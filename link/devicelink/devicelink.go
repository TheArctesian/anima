// Package devicelink is the device side of the link: a loopback port that
// reaches the hub, and a keepalive that holds the link ready.
package devicelink

import (
	"context"
	"errors"
	"net"
	"net/netip"
	"sync"
	"time"

	"github.com/TheArctesian/anima/link/identity"
	"github.com/TheArctesian/anima/link/tunnel"
)

// Errors of this package.
var (
	// ErrListen means that the local address is not on loopback. The local
	// port gives the hub to whoever can reach it, so it stays on this computer.
	ErrListen = errors.New("devicelink: the local address must be a loopback address and a port, for example 127.0.0.1:0")
	// ErrNotEnrolled means that the device has no hub address yet.
	ErrNotEnrolled = errors.New("devicelink: this device has no hub; it needs an invite")
)

// Times of the device link.
const (
	// DefaultKeepalive is how often the device pings the hub. A link with no
	// traffic for 150 s or more stalls for seconds at the next request
	// (spike K-A, 3.6); 15 s keeps it ready.
	DefaultKeepalive = 15 * time.Second
	pingLimit        = 5 * time.Second
	registerLimit    = 10 * time.Second
	dialLimit        = 15 * time.Second
)

// Config is what a device link needs to start.
type Config struct {
	Identity identity.Device
	// Listen is the loopback address of the local port. Port 0 takes a free port.
	Listen string
	// Keepalive is the time between pings. Zero means DefaultKeepalive.
	Keepalive time.Duration
	DebugLogf tunnel.Logf
}

// Link is a running device link.
type Link struct {
	device   *tunnel.Device
	listener net.Listener
	stop     context.CancelFunc
	done     sync.WaitGroup

	mu   sync.Mutex
	path tunnel.Path
}

// Start opens the local port and starts the keepalive. It does not wait for
// the hub: a hub that is off now is reached when it comes back.
func Start(cfg Config) (*Link, error) {
	local, err := netip.ParseAddrPort(cfg.Listen)
	if err != nil || !local.Addr().IsLoopback() {
		return nil, ErrListen
	}
	if cfg.Identity.Hub.IsZero() {
		return nil, ErrNotEnrolled
	}
	device, err := tunnel.NewDevice(tunnel.DeviceConfig{Key: cfg.Identity.Key, Hub: cfg.Identity.Hub, DebugLogf: cfg.DebugLogf})
	if err != nil {
		return nil, err
	}
	listener, err := net.Listen("tcp", local.String())
	if err != nil {
		device.Close()
		return nil, err
	}
	ctx, stop := context.WithCancel(context.Background())
	link := &Link{device: device, listener: listener, stop: stop, path: tunnel.Path{Kind: tunnel.PathNone}}
	every := cfg.Keepalive
	if every <= 0 {
		every = DefaultKeepalive
	}
	link.done.Add(2)
	go link.keepAlive(ctx, every)
	go link.accept(ctx)
	return link, nil
}

// LocalAddr is the local port that reaches the hub.
func (l *Link) LocalAddr() net.Addr { return l.listener.Addr() }

// Path is the result of the last keepalive (LNK-006).
func (l *Link) Path() tunnel.Path {
	l.mu.Lock()
	defer l.mu.Unlock()
	return l.path
}

// Close stops the device link and its streams.
func (l *Link) Close() error {
	l.stop()
	l.listener.Close()
	err := l.device.Close()
	l.done.Wait()
	return err
}

// keepAlive pings the hub at once and then at each interval. The ping holds
// the link in use. When the hub does not answer, the device registers again:
// a hub that restarted has forgotten it, and the library registers only at
// the first use (spike K-A, 3.2).
func (l *Link) keepAlive(ctx context.Context, every time.Duration) {
	defer l.done.Done()
	ticker := time.NewTicker(every)
	defer ticker.Stop()
	for {
		l.beat(ctx)
		select {
		case <-ctx.Done():
			return
		case <-ticker.C:
		}
	}
}

func (l *Link) beat(ctx context.Context) {
	path, err := l.ping(ctx)
	if err != nil {
		registerCtx, cancel := context.WithTimeout(ctx, registerLimit)
		registered := l.device.Register(registerCtx) == nil
		cancel()
		if registered {
			path, _ = l.ping(ctx)
		}
	}
	l.mu.Lock()
	l.path = path
	l.mu.Unlock()
}

func (l *Link) ping(ctx context.Context) (tunnel.Path, error) {
	ctx, cancel := context.WithTimeout(ctx, pingLimit)
	defer cancel()
	return l.device.Path(ctx)
}

func (l *Link) accept(ctx context.Context) {
	defer l.done.Done()
	for {
		local, err := l.listener.Accept()
		if err != nil {
			return
		}
		go l.forward(ctx, local)
	}
}

// forward sends one local connection to the hub.
func (l *Link) forward(ctx context.Context, local net.Conn) {
	ctx, cancel := context.WithTimeout(ctx, dialLimit)
	defer cancel()
	hub, err := l.device.Dial(ctx, tunnel.HubPort)
	if err != nil {
		local.Close()
		return
	}
	tunnel.Join(local, hub)
}
