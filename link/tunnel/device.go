package tunnel

import (
	"context"
	"fmt"
	"net"
	"time"

	"github.com/tailscale/tailcat"
)

// DeviceConfig is what the device side needs.
type DeviceConfig struct {
	Key PrivateKey
	Hub Address
	// DebugLogf, if set, gets the library's log.
	DebugLogf Logf
}

// Device is the device side of the link. It connects at its first use.
type Device struct{ client *tailcat.Client }

// NewDevice prepares the device side. It opens no connection.
func NewDevice(cfg DeviceConfig) (*Device, error) {
	if cfg.Key.IsZero() {
		return nil, ErrNoIdentity
	}
	if _, err := tailcat.ParseAddr(tailcat.Addr(cfg.Hub)); err != nil {
		return nil, ErrBadAddress
	}
	return &Device{client: &tailcat.Client{
		Server: tailcat.Addr(cfg.Hub),
		Key:    cfg.Key.node,
		Logf:   cfg.DebugLogf.orDiscard(),
	}}, nil
}

// Dial opens one stream to a port of the hub.
func (d *Device) Dial(ctx context.Context, port uint16) (net.Conn, error) {
	return d.client.DialTCPPort(ctx, port)
}

// Register announces the device to the hub. The library does this only at
// the first use, and a hub that restarted has forgotten the device, so a
// device calls it again when the hub does not answer.
func (d *Device) Register(ctx context.Context) error {
	_, err := d.client.Ping(ctx)
	return err
}

// Path pings the hub and reports how the answer came back. The ping also
// starts the search for a direct path.
func (d *Device) Path(ctx context.Context) (Path, error) {
	result, err := d.client.DiscoPing(ctx)
	if err != nil {
		return Path{Kind: PathNone}, fmt.Errorf("tunnel: ping: %w", err)
	}
	latency := time.Duration(result.LatencySeconds * float64(time.Second))
	if result.Endpoint != "" {
		return Path{Kind: PathDirect, Latency: latency}, nil
	}
	return Path{Kind: PathRelayed, Latency: latency}, nil
}

// Close stops the device side.
func (d *Device) Close() error { return d.client.Close() }
