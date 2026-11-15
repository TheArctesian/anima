package tunnel

import (
	"context"
	"errors"
	"fmt"
	"net"

	"github.com/tailscale/tailcat"
	"tailscale.com/types/key"
	"tailscale.com/types/logger"
)

// Errors of this package.
var (
	ErrNoAdmit    = errors.New("tunnel: a hub needs an Admit function")
	ErrNoIdentity = errors.New("tunnel: the key or the pre-shared key is not set")
	ErrStart      = errors.New("tunnel: the link did not start")
	ErrBadAddress = errors.New("tunnel: the hub address is not valid")
)

// startError is ErrStart with the library's cause. Its text is fixed: the
// library's text can name relay hosts and URLs (LNK-008). The cause stays
// reachable with errors.Is and errors.As.
type startError struct{ cause error }

func (startError) Error() string     { return ErrStart.Error() }
func (e startError) Unwrap() []error { return []error{ErrStart, e.cause} }

// Logf is the library's log sink. It can get text that names keys and
// network addresses, so it must not go to a normal log (LNK-008).
type Logf func(format string, args ...any)

func (l Logf) orDiscard() logger.Logf {
	if l == nil {
		return logger.Discard
	}
	return logger.Logf(l)
}

// HubConfig is what the hub side needs to start.
type HubConfig struct {
	Key          PrivateKey
	PresharedKey PresharedKey
	// Relay is the saved relay. Zero at the first start only: the hub then
	// takes the nearest relay of the map at RelayMapURL.
	Relay Relay
	// RelayMapURL is where the relay map is when Relay is zero. Empty means
	// Tailscale's map of shared relays.
	RelayMapURL string
	// Admit decides if a device key can connect. It is asked when a device
	// that is not connected announces itself, and not again while it stays
	// connected (LNK-002).
	Admit func(DeviceKey) bool
	// DebugLogf, if set, gets the library's log.
	DebugLogf Logf
}

// Hub is the hub side of the link.
type Hub struct {
	server *tailcat.Server
	relay  Relay
}

// StartHub connects the hub side to its relay. ctx limits the start only.
func StartHub(ctx context.Context, cfg HubConfig) (*Hub, error) {
	if cfg.Admit == nil {
		return nil, ErrNoAdmit
	}
	if cfg.Key.IsZero() || cfg.PresharedKey.IsZero() {
		return nil, ErrNoIdentity
	}
	server := &tailcat.Server{
		Key:          cfg.Key.node,
		PresharedKey: cfg.PresharedKey.psk,
		Region:       cfg.Relay.region,
		DERPMapURL:   cfg.RelayMapURL,
		Logf:         cfg.DebugLogf.orDiscard(),
		AllowClient:  func(k key.NodePublic) bool { return cfg.Admit(DeviceKey{k}) },
	}
	if err := server.StartContext(ctx); err != nil {
		return nil, startError{err}
	}
	relay := cfg.Relay
	if relay.IsZero() {
		parsed, err := tailcat.ParseAddr(server.TailcatAddr())
		if err != nil || len(parsed.Region) == 0 {
			server.Close()
			return nil, fmt.Errorf("%w: the chosen relay cannot be read", ErrStart)
		}
		relay = Relay{parsed.Region[0]}
	}
	return &Hub{server: server, relay: relay}, nil
}

// Address is the address that devices use. It is a secret (LNK-008).
func (h *Hub) Address() Address { return Address{text: h.server.TailcatAddr()} }

// Relay is the relay in use. Save it and give it back at the next start, or
// the address changes (LNK-001).
func (h *Hub) Relay() Relay { return h.relay }

// Listen accepts streams that devices open to one port of the hub.
func (h *Hub) Listen(port uint16) (net.Listener, error) {
	return h.server.Listen(context.Background(), "tcp", fmt.Sprintf(":%d", port))
}

// DeviceOf returns the device key that the tunnel authenticated for the
// remote address of an accepted stream.
func (h *Hub) DeviceOf(remote net.Addr) (DeviceKey, bool) {
	k, ok := h.server.PeerKey(remote)
	return DeviceKey{k}, ok
}

// Disconnect makes the hub forget a connected device and stops its traffic.
// It does not close the device's open streams: close them first (LNK-004).
// Admit must already refuse the key, or the device connects again.
func (h *Hub) Disconnect(device DeviceKey) bool { return h.server.DisconnectClient(device.node) }

// Paths reports the path of each device that the library knows. A device
// that was disconnected can stay in the result with PathNone.
func (h *Hub) Paths() map[DeviceKey]Path {
	paths := map[DeviceKey]Path{}
	for k, peer := range h.server.Status().Peer {
		switch {
		case peer.CurAddr != "":
			paths[DeviceKey{k}] = Path{Kind: PathDirect}
		case peer.Relay != "":
			paths[DeviceKey{k}] = Path{Kind: PathRelayed, Relay: peer.Relay}
		default:
			paths[DeviceKey{k}] = Path{Kind: PathNone}
		}
	}
	return paths
}

// Close stops the hub side.
func (h *Hub) Close() error { return h.server.Close() }

// The ports of the link.
const (
	// HubPort carries the hub's own traffic.
	HubPort uint16 = 443
	// EnrolPort is where a device with an invite gives its key (LNK-003).
	EnrolPort uint16 = 444
)

// Join copies bytes between two streams in both directions until both have
// ended, then closes both. When one direction ends, the other stays open, so
// a side can end its request and still read the answer.
func Join(a, b net.Conn) { tailcat.ProxyConns(a, b) }
