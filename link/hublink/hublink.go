// Package hublink is the hub side of the link: it accepts streams from
// permitted devices and sends each one to the hub on loopback.
package hublink

import (
	"context"
	"errors"
	"io"
	"log/slog"
	"net"
	"net/netip"
	"sync"
	"sync/atomic"
	"time"

	"github.com/TheArctesian/anima/link/devices"
	"github.com/TheArctesian/anima/link/identity"
	"github.com/TheArctesian/anima/link/invites"
	"github.com/TheArctesian/anima/link/tunnel"
)

// ErrTarget means that the target is not a loopback address with a port.
// The link must not send a device's traffic to another computer.
var ErrTarget = errors.New("hublink: the target must be a loopback address and a port, for example 127.0.0.1:8443")

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
	// Target is the hub's loopback address.
	Target string
	// Devices is the list of permitted devices (LNK-002). It is asked when a
	// device connects and again for each stream.
	Devices *devices.Store
	// Invites lets devices join with an invite (LNK-003). Nil means that no
	// device can join.
	Invites *invites.Store
	// Now is the clock for invites. Nil means the system clock.
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
	target  string
	log     *slog.Logger
	devices *devices.Store
	invites *invites.Store
	now     func() time.Time
	streams streams

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
	target, err := netip.ParseAddrPort(cfg.Target)
	if err != nil || !target.Addr().IsLoopback() || target.Port() == 0 {
		return nil, ErrTarget
	}
	link := newLink(cfg, target.String())
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
	serve := map[uint16]func(net.Conn){tunnel.HubPort: link.forward, tunnel.NoticePort: link.notify}
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

func newLink(cfg Config, target string) *Link {
	link := &Link{
		target: target, log: cfg.Log, devices: cfg.Devices, invites: cfg.Invites, now: cfg.Now,
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
	if !l.devices.Permits(device) {
		stream.Close()
		return
	}
	hub, err := net.DialTimeout("tcp", l.target, targetDialLimit)
	if err != nil {
		l.log.Warn("the hub did not accept a connection on its loopback address", "device", device)
		stream.Close()
		return
	}
	tunnel.Join(stream, hub)
}
