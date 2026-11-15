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
	"time"

	"github.com/TheArctesian/anima/link/identity"
	"github.com/TheArctesian/anima/link/tunnel"
)

// Errors of this package.
var (
	// ErrStillPermitted means that Disconnect was called for a device that
	// is in the list. A disconnected device that is permitted connects again
	// at once, so the key must leave the list first (LNK-004).
	ErrStillPermitted = errors.New("hublink: remove the device from the list before it is disconnected")
	// ErrTarget means that the target is not a loopback address with a port.
	// The link must not send a device's traffic to another computer.
	ErrTarget = errors.New("hublink: the target must be a loopback address and a port, for example 127.0.0.1:8443")
)

const (
	targetDialLimit = 5 * time.Second
	// closeGrace is the time that the close of a stream gets to reach the
	// device before the tunnel stops carrying the device's traffic. The
	// tunnel gives no sign that the close arrived.
	closeGrace = 500 * time.Millisecond
)

// Config is what a hub link needs to start.
type Config struct {
	Identity identity.Hub
	// Target is the hub's loopback address.
	Target string
	// Permits answers if a device key can connect (LNK-002). It is asked
	// when a device connects, not for each stream: a connected device stays
	// connected until the hub link disconnects it.
	Permits func(tunnel.DeviceKey) bool
	// Log gets what the operator must know. Nil means no log.
	Log *slog.Logger
	// RelayMapURL is where the relay map is at the first start. Empty means
	// Tailscale's map of shared relays.
	RelayMapURL string
	DebugLogf   tunnel.Logf
}

// Link is a running hub link.
type Link struct {
	hub      *tunnel.Hub
	listener net.Listener
	target   string
	log      *slog.Logger
	permits  func(tunnel.DeviceKey) bool
	streams  streams
	accepted chan struct{}
}

// Start connects the hub link to its relay and starts to accept streams.
// ctx limits the start only.
func Start(ctx context.Context, cfg Config) (*Link, error) {
	target, err := netip.ParseAddrPort(cfg.Target)
	if err != nil || !target.Addr().IsLoopback() || target.Port() == 0 {
		return nil, ErrTarget
	}
	hub, err := tunnel.StartHub(ctx, tunnel.HubConfig{
		Key:          cfg.Identity.Key,
		PresharedKey: cfg.Identity.PresharedKey,
		Relay:        cfg.Identity.Relay,
		RelayMapURL:  cfg.RelayMapURL,
		Admit:        cfg.Permits,
		DebugLogf:    cfg.DebugLogf,
	})
	if err != nil {
		return nil, err
	}
	listener, err := hub.Listen(tunnel.HubPort)
	if err != nil {
		hub.Close()
		return nil, err
	}
	log := cfg.Log
	if log == nil {
		log = slog.New(slog.NewTextHandler(io.Discard, nil))
	}
	link := &Link{hub: hub, listener: listener, target: target.String(), log: log, permits: cfg.Permits, accepted: make(chan struct{})}
	go link.accept()
	return link, nil
}

// Address is the address that devices use. It is a secret (LNK-008).
func (l *Link) Address() tunnel.Address { return l.hub.Address() }

// Relay is the relay in use. The caller saves it with the identity (LNK-001).
func (l *Link) Relay() tunnel.Relay { return l.hub.Relay() }

// Streams is the number of open streams of one device.
func (l *Link) Streams(device tunnel.DeviceKey) int { return l.streams.count(device) }

// Disconnect ends the open sessions of a device that left the list
// (LNK-004), and returns how many streams it closed. It closes each stream
// of the device, so that the device sees its sessions end, and then makes
// the tunnel forget the device. The tunnel's own call does not close
// streams: without the first step the sessions stay open with no traffic.
func (l *Link) Disconnect(device tunnel.DeviceKey) (closed int, err error) {
	if l.permits(device) {
		return 0, ErrStillPermitted
	}
	closed = l.streams.closeOf(device)
	if closed > 0 {
		time.Sleep(closeGrace)
	}
	l.hub.Disconnect(device)
	l.log.Info("a device was disconnected", "device", device, "streams", closed)
	return closed, nil
}

// Close stops the hub link. It returns when each stream and each
// connection to the hub is closed.
func (l *Link) Close() error {
	l.streams.closing()
	l.listener.Close()
	<-l.accepted
	l.streams.closeAll()
	return l.hub.Close()
}

func (l *Link) accept() {
	defer close(l.accepted)
	for {
		stream, err := l.listener.Accept()
		if err != nil {
			if !l.streams.isClosing() {
				l.log.Error("the hub link accepts no more streams", "cause", "the link's listener ended")
			}
			return
		}
		go l.forward(stream)
	}
}

// forward sends one stream to the hub. The stream is recorded under the
// device key that the tunnel authenticated.
func (l *Link) forward(stream net.Conn) {
	device, known := l.hub.DeviceOf(stream.RemoteAddr())
	// The list is asked again for each stream: the tunnel asks it only when
	// a device connects, and a device can leave the list while connected.
	if !known || !l.permits(device) {
		stream.Close()
		return
	}
	if !l.streams.add(device, stream) {
		l.log.Warn("a stream was refused: the hub link has stopped or a stream limit is reached", "device", device)
		stream.Close()
		return
	}
	defer l.streams.remove(device, stream)
	hub, err := net.DialTimeout("tcp", l.target, targetDialLimit)
	if err != nil {
		l.log.Warn("the hub did not accept a connection on its loopback address", "device", device)
		stream.Close()
		return
	}
	tunnel.Join(stream, hub)
}
