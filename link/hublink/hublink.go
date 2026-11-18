// Package hublink is the hub side of the link: it accepts streams from
// permitted devices and sends each one to the hub on loopback.
package hublink

import (
	"context"
	"errors"
	"io"
	"io/fs"
	"log/slog"
	"net"
	"net/netip"
	"sync"
	"sync/atomic"
	"time"

	"github.com/TheArctesian/anima/link/devices"
	"github.com/TheArctesian/anima/link/dial"
	"github.com/TheArctesian/anima/link/identity"
	"github.com/TheArctesian/anima/link/invites"
	"github.com/TheArctesian/anima/link/presence"
	"github.com/TheArctesian/anima/link/statedir"
	"github.com/TheArctesian/anima/link/tunnel"
)

// Times of the hub link.
const (
	targetDialLimit = 5 * time.Second
	// closeGrace is the time that the close of a stream gets to reach the
	// device before the tunnel stops carrying the device's traffic. The
	// tunnel gives no sign that the close arrived.
	closeGrace = 500 * time.Millisecond
	// DefaultSweep is how often connected devices are checked against the
	// list. A key that expires, or a revocation that was cut before its
	// disconnect, ends within this time (LNK-004, LNK-005).
	DefaultSweep = 30 * time.Second
)

// Config is what a hub link needs to start.
type Config struct {
	Identity identity.Hub
	// Target is the path of the hub's socket for the link, or a loopback
	// address. Only a socket gets the header that names the device (see
	// deviceheader.go): to a loopback address every device looks the same.
	Target string
	// Devices is the list of permitted devices (LNK-002). It is asked when a
	// device connects and again for each stream.
	Devices *devices.Store
	// Invites lets devices join with an invite (LNK-003). Nil means that no
	// device can join.
	Invites *invites.Store
	// Now is the clock for invites and for the time that a device was last
	// seen. Nil means the system clock.
	Now func() time.Time
	// RelayMapURL is where the relay map is at the first start. Empty means
	// Tailscale's map of shared relays.
	RelayMapURL string
	// Sweep is how often the hub link disconnects devices that are
	// connected but not permitted any more. Zero means DefaultSweep.
	Sweep time.Duration
	// Log gets what the operator must know. Nil means no log.
	Log       *slog.Logger
	DebugLogf tunnel.Logf
}

// Link is a running hub link.
type Link struct {
	hub     *tunnel.Hub
	ownKey  tunnel.DeviceKey
	target  target
	log     *slog.Logger
	devices *devices.Store
	invites *invites.Store
	now     func() time.Time
	streams streams
	// present is what the devices say on the reverse port (HMD-047).
	present presence.Book
	// asked is the requests for a local port of a device (HMD-048).
	asked dial.Desk

	// enrolling limits the enrolments that run at one time; enrolments waits
	// for them when the hub link stops.
	enrolling  chan struct{}
	enrolments sync.WaitGroup

	identity identity.Hub
	// next is the address that devices are told for after a relay change.
	next atomic.Pointer[tunnel.Address]

	listeners []net.Listener
	closeOnce sync.Once
	// stopped closes when Close begins.
	stopped   chan struct{}
	accepting chan struct{}
}

// Start connects the hub link to its relay and starts to accept streams.
// ctx limits the start only.
func Start(ctx context.Context, cfg Config) (*Link, error) {
	to, err := parseTarget(cfg.Target)
	if err != nil {
		return nil, err
	}
	link := newLink(cfg, to)
	link.hub, err = tunnel.StartHub(ctx, tunnel.HubConfig{
		Key:          cfg.Identity.Key,
		PresharedKey: cfg.Identity.PresharedKey,
		Relay:        cfg.Identity.Relay,
		RelayMapURL:  cfg.RelayMapURL,
		Admit:        link.admitted,
		DebugLogf:    cfg.DebugLogf,
	})
	if err != nil {
		return nil, err
	}
	serve := map[uint16]func(net.Conn){tunnel.HubPort: link.forward, tunnel.NoticePort: link.notify, tunnel.ReversePort: link.hear, tunnel.OpenPort: link.give}
	if link.invites != nil {
		serve[tunnel.EnrolPort] = link.enrol
	}
	for port, handle := range serve {
		listener, err := link.hub.Listen(port)
		if err != nil {
			link.Close()
			return nil, err
		}
		link.listeners = append(link.listeners, listener)
		go link.accept(listener, handle)
	}
	sweep := cfg.Sweep
	if sweep <= 0 {
		sweep = DefaultSweep
	}
	go link.sweep(sweep)
	return link, nil
}

func newLink(cfg Config, to target) *Link {
	link := &Link{
		target: to, log: cfg.Log, devices: cfg.Devices, invites: cfg.Invites, now: cfg.Now,
		ownKey: cfg.Identity.Key.Public(), enrolling: make(chan struct{}, maxEnrolments), identity: cfg.Identity,
		stopped: make(chan struct{}), accepting: make(chan struct{}, 2),
	}
	if link.log == nil {
		link.log = slog.New(slog.NewTextHandler(io.Discard, nil))
	}
	if link.now == nil {
		link.now = time.Now
	}
	if !cfg.Identity.NextRelay.IsZero() {
		link.Announce(cfg.Identity.NextRelay)
	}
	return link
}

// Address is the address that devices use. It is a secret (LNK-008).
func (l *Link) Address() tunnel.Address { return l.hub.Address() }

// Relay is the relay in use. The caller saves it with the identity (LNK-001).
func (l *Link) Relay() tunnel.Relay { return l.hub.Relay() }

// Path reports how the traffic of one device reaches the hub now, with the
// relay's name when it is relayed (LNK-006). A device that is not connected
// has no path.
func (l *Link) Path(device tunnel.DeviceKey) tunnel.Path {
	if path, known := l.hub.Paths()[device]; known {
		return path
	}
	return tunnel.Path{Kind: tunnel.PathNone}
}

// Streams is the number of open streams of one device.
func (l *Link) Streams(device tunnel.DeviceKey) int { return l.streams.count(device) }

// Close stops the hub link. It returns when each stream and each
// connection to the hub is closed. A second call does nothing.
func (l *Link) Close() error {
	var err error
	l.closeOnce.Do(func() {
		l.streams.closing()
		close(l.stopped)
		for _, listener := range l.listeners {
			listener.Close()
		}
		for range l.listeners {
			<-l.accepting
		}
		l.streams.closeAll()
		l.enrolments.Wait()
		err = l.hub.Close()
	})
	return err
}

func (l *Link) accept(listener net.Listener, handle func(net.Conn)) {
	defer func() { l.accepting <- struct{}{} }()
	for {
		stream, err := listener.Accept()
		if err != nil {
			if !l.streams.isClosing() {
				l.log.Error("the hub link accepts no more streams", "cause", "a listener of the link ended")
			}
			return
		}
		go handle(stream)
	}
}

// forward sends one stream to the hub. The stream is recorded under the
// device key that the tunnel authenticated.
func (l *Link) forward(stream net.Conn) {
	device, known := l.hub.DeviceOf(stream.RemoteAddr())
	if !known {
		stream.Close()
		return
	}
	if !l.streams.add(device, stream) {
		l.log.Warn("a stream was refused: the hub link has stopped or a stream limit is reached", "device", device)
		stream.Close()
		return
	}
	defer l.streams.remove(device, stream)
	// The list is asked for each stream, after the stream is recorded: the
	// tunnel asks the list only when a device connects, and a device can
	// leave the list while connected. A revocation removes the key and then
	// closes the recorded streams, so a stream is either closed by it or
	// refused here.
	address, permitted := l.devices.AddressOf(device)
	if !permitted {
		stream.Close()
		return
	}
	hub, err := l.target.dial()
	if err != nil {
		l.log.Warn("the hub did not accept a connection from the link", "device", device, "cause", dialFault(err))
		stream.Close()
		return
	}
	if l.target.socket != "" && !named(hub, address, stream) {
		stream.Close()
		hub.Close()
		return
	}
	tunnel.Join(stream, hub)
}

// Seen is the time of the last "here" line of the device that has a link
// address (HMD-047). It reports false for a device that wrote none since
// the hub link started.
func (l *Link) Seen(device netip.Addr) (time.Time, bool) { return l.present.Seen(device) }

// hear serves one stream of the reverse port. The stream is recorded under
// the device key that the tunnel authenticated and counts as a stream of its
// device, so a revocation closes it and the stream limits hold. The list is
// asked after the stream is recorded, as in forward.
func (l *Link) hear(stream net.Conn) {
	defer stream.Close()
	key, known := l.hub.DeviceOf(stream.RemoteAddr())
	if !known || !l.streams.add(key, stream) {
		return
	}
	defer l.streams.remove(key, stream)
	device, permitted := l.devices.AddressOf(key)
	if !permitted {
		return
	}
	if replaced := l.present.Hold(device, stream); replaced != nil {
		replaced.Close()
	}
	defer l.present.Release(device, stream)
	presence.Read(stream, func() { l.present.Saw(device, l.now()) })
}

// Open asks the device that has a link address for one of its local ports
// and returns the stream to that port (HMD-048). The device decides: it
// gives only a port that its person opened. The caller calls release when
// it has finished with the stream. The errors are those of package dial.
func (l *Link) Open(ctx context.Context, device netip.Addr, port uint16) (stream net.Conn, release func(), err error) {
	return l.asked.Open(ctx, device, port, func(line string) error { return l.present.Write(device, line) })
}

// give serves one stream of the open port: the answer of a device to a
// request of Open. The stream is recorded and the list is asked as in hear.
func (l *Link) give(stream net.Conn) {
	defer stream.Close()
	key, known := l.hub.DeviceOf(stream.RemoteAddr())
	if !known || !l.streams.add(key, stream) {
		return
	}
	defer l.streams.remove(key, stream)
	device, permitted := l.devices.AddressOf(key)
	if !permitted {
		return
	}
	l.asked.Deliver(device, stream)
}

// named writes the header that names the device to the hub, before any
// byte of the device.
func named(hub net.Conn, device netip.Addr, stream net.Conn) bool {
	header := deviceHeader(device, addrPortOf(stream.RemoteAddr()).Port(), addrPortOf(stream.LocalAddr()))
	_ = hub.SetWriteDeadline(time.Now().Add(targetDialLimit))
	_, err := hub.Write(header)
	_ = hub.SetWriteDeadline(time.Time{})
	return err == nil
}

// dialFault says in a fixed word why the hub could not be reached, for the
// log: the error's own text holds a path.
func dialFault(err error) string {
	if errors.Is(err, statedir.ErrExposed) {
		return "other users can reach the hub's socket or its folder"
	}
	if errors.Is(err, fs.ErrNotExist) {
		return "the hub's socket is not there; the hub makes it when it starts"
	}
	return "no answer"
}
