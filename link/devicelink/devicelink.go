// Package devicelink is the device side of the link: a loopback port and,
// for the metrics daemon, a Unix socket that reach the hub, a keepalive that holds the link ready, and a stream that
// tells the hub that the device is connected and gives the hub the local
// ports that the person opened.
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
	// DefaultNoticeEvery makes the device read the hub's next address each
	// 5 minutes with the default keepalive.
	DefaultNoticeEvery = 20
	// DefaultAnswerLimit is how long the device waits for one answer of the
	// hub before it counts the hub as silent.
	DefaultAnswerLimit = 5 * time.Second
	dialLimit          = 15 * time.Second
)

// Config is what a device link needs to start.
type Config struct {
	Identity identity.Device
	// Listen is the loopback address of the local port. Port 0 takes a free port.
	Listen string
	// DaemonSocket is a second way to the hub on this computer: a Unix
	// socket for the metrics daemon, which sends its token and must not
	// send it to a port (K8 Section 1). The link closes it. Nil means none.
	DaemonSocket net.Listener
	// Keepalive is the time between pings. Zero means DefaultKeepalive.
	Keepalive time.Duration
	// AnswerLimit is how long the device waits for one answer of the hub. A
	// registration, which is two messages, gets twice this. Zero means
	// DefaultAnswerLimit.
	AnswerLimit time.Duration
	// Save keeps the device's identity when a renewal changes it (LNK-005).
	// Nil means that the key is never renewed.
	Save func(identity.Device) error
	// NoticeEvery is how many keepalives pass between two reads of the
	// hub's next address (LNK-007). Zero means DefaultNoticeEvery.
	NoticeEvery int
	// LookupTXT reads the TXT records of a domain name, for a device whose
	// identity has a HubName (LNK-007). Nil means the system's resolver.
	LookupTXT func(ctx context.Context, name string) ([]string, error)
	// OpenPort reports whether the hub may reach a local port of this
	// computer through the link (HMD-048, HMD-049). It is asked for each
	// request. Nil means that no port is open.
	OpenPort func(port uint16) bool
	// Now is the clock. Nil means the system clock.
	Now       func() time.Time
	DebugLogf tunnel.Logf
}

// Link is a running device link.
type Link struct {
	listener  net.Listener
	daemon    net.Listener // Config.DaemonSocket; nil when there is none
	stop      context.CancelFunc
	done      sync.WaitGroup
	save      func(identity.Device) error
	lookupTXT func(ctx context.Context, name string) ([]string, error)
	now       func() time.Time
	debugLogf tunnel.Logf

	// noticeEvery is how many keepalives pass between two reads of the
	// hub's notice. beats and failures belong to the keepalive goroutine.
	noticeEvery, beats, failures int
	// pingLimit and registerLimit come from Config.AnswerLimit.
	pingLimit, registerLimit time.Duration

	mu     sync.Mutex
	device *tunnel.Device // replaced when the key is renewed or the hub moves
	saved  identity.Device
	path   tunnel.Path
	// reverse is the open stream to the reverse port, if there is one.
	reverse net.Conn
	// presentNow asks for a new stream to the reverse port with no wait.
	presentNow chan struct{}
	// openPort is Config.OpenPort; giving holds one value for each
	// connection to a local port that the hub has now.
	openPort func(port uint16) bool
	giving   chan struct{}
}

// tunnel is the device side of the tunnel for the device's current key.
func (l *Link) tunnel() *tunnel.Device {
	l.mu.Lock()
	defer l.mu.Unlock()
	return l.device
}

func (l *Link) identity() identity.Device {
	l.mu.Lock()
	defer l.mu.Unlock()
	return l.saved
}

func (l *Link) setIdentity(device identity.Device) {
	l.mu.Lock()
	defer l.mu.Unlock()
	l.saved = device
}

// replaceTunnel makes a renewed key the key in use and returns the tunnel
// of the old key.
func (l *Link) replaceTunnel(next *tunnel.Device, renewed identity.Device) (old *tunnel.Device) {
	l.mu.Lock()
	defer l.mu.Unlock()
	old, l.device, l.saved = l.device, next, renewed
	return old
}

// Start opens the local port and starts the keepalive and the stream to the
// reverse port. It does not wait for the hub: a hub that is off now is
// reached when it comes back. A start that fails closes Config.DaemonSocket.
func Start(cfg Config) (*Link, error) {
	link, err := start(cfg)
	if err != nil && cfg.DaemonSocket != nil {
		cfg.DaemonSocket.Close()
	}
	return link, err
}

func start(cfg Config) (*Link, error) {
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
	link := &Link{
		device: device, saved: cfg.Identity, listener: listener, daemon: cfg.DaemonSocket, stop: stop, path: tunnel.Path{Kind: tunnel.PathNone},
		save: cfg.Save, now: cfg.Now, debugLogf: cfg.DebugLogf, presentNow: make(chan struct{}, 1),
		openPort: cfg.OpenPort, giving: make(chan struct{}, maxLocalConnections),
	}
	if link.now == nil {
		link.now = time.Now
	}
	link.lookupTXT = cfg.LookupTXT
	if link.lookupTXT == nil {
		link.lookupTXT = net.DefaultResolver.LookupTXT
	}
	link.pingLimit = cfg.AnswerLimit
	if link.pingLimit <= 0 {
		link.pingLimit = DefaultAnswerLimit
	}
	link.registerLimit = 2 * link.pingLimit
	link.noticeEvery = max(cfg.NoticeEvery, 0)
	if link.noticeEvery == 0 {
		link.noticeEvery = DefaultNoticeEvery
	}
	every := cfg.Keepalive
	if every <= 0 {
		every = DefaultKeepalive
	}
	link.done.Add(3)
	go link.keepAlive(ctx, every)
	go link.stayPresent(ctx, every)
	go link.accept(ctx, listener)
	if link.daemon != nil {
		link.done.Add(1)
		go link.accept(ctx, link.daemon)
	}
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
	if l.daemon != nil {
		l.daemon.Close()
	}
	err := l.tunnel().Close()
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
		registerCtx, cancel := context.WithTimeout(ctx, l.registerLimit)
		registered := l.tunnel().Register(registerCtx) == nil
		cancel()
		if registered {
			path, _ = l.ping(ctx)
			l.presentAgain()
		}
	}
	l.mu.Lock()
	l.path = path
	l.mu.Unlock()
	if path.Kind == tunnel.PathNone {
		l.failures++
		if l.failures >= failuresBeforeNextHub {
			l.followHub(ctx)
		}
		return
	}
	l.failures = 0
	l.renewIfDue(ctx)
	if l.beats%l.noticeEvery == 0 {
		l.readNotice(ctx)
	}
	l.beats++
}

func (l *Link) ping(ctx context.Context) (tunnel.Path, error) {
	ctx, cancel := context.WithTimeout(ctx, l.pingLimit)
	defer cancel()
	return l.tunnel().Path(ctx)
}

// accept sends each connection of one listener, the local port or the
// socket of the daemon, to the hub.
func (l *Link) accept(ctx context.Context, listener net.Listener) {
	defer l.done.Done()
	for {
		local, err := listener.Accept()
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
	hub, err := l.tunnel().Dial(ctx, tunnel.HubPort)
	if err != nil {
		local.Close()
		return
	}
	tunnel.Join(local, hub)
}
