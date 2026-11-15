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
	// ErrTarget means that the target is not a loopback address with a port.
	// The link must not send a device's traffic to another computer.
	ErrTarget = errors.New("hublink: the target must be a loopback address and a port, for example 127.0.0.1:8443")
)

const targetDialLimit = 5 * time.Second

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
	link := &Link{hub: hub, listener: listener, target: target.String(), log: log, accepted: make(chan struct{})}
	go link.accept()
	return link, nil
}

// Address is the address that devices use. It is a secret (LNK-008).
func (l *Link) Address() tunnel.Address { return l.hub.Address() }

// Relay is the relay in use. The caller saves it with the identity (LNK-001).
func (l *Link) Relay() tunnel.Relay { return l.hub.Relay() }

// Streams is the number of open streams of one device.
func (l *Link) Streams(device tunnel.DeviceKey) int { return l.streams.count(device) }

// Close stops the hub link. It returns when each stream and each
// connection to the hub is closed.
func (l *Link) Close() error {
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
			return
		}
		go l.forward(stream)
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
