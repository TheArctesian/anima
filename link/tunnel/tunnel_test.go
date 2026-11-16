package tunnel_test

import (
	"bytes"
	"context"
	"io"
	"net"
	"sync/atomic"
	"testing"
	"time"

	"github.com/TheArctesian/anima/link/tunnel"
	"github.com/TheArctesian/anima/link/tunnel/testbed"
)

func TestMain(m *testing.M) { testbed.Main(m) }

const echoPort = 7

// identity is what a hub saves between starts.
type identity struct {
	key   tunnel.PrivateKey
	psk   tunnel.PresharedKey
	relay tunnel.Relay
}

func newIdentity(t *testing.T) identity {
	return identity{tunnel.NewPrivateKey(), tunnel.NewPresharedKey(), testbed.StartRelay(t)}
}

func startHub(t *testing.T, id identity, admit func(tunnel.DeviceKey) bool) *tunnel.Hub {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), 20*time.Second)
	defer cancel()
	hub, err := tunnel.StartHub(ctx, tunnel.HubConfig{Key: id.key, PresharedKey: id.psk, Relay: id.relay, Admit: admit})
	if err != nil {
		t.Fatalf("StartHub: %v", err)
	}
	t.Cleanup(func() { hub.Close() })
	return hub
}

// serveEcho answers each stream on echoPort with the bytes it gets.
func serveEcho(t *testing.T, hub *tunnel.Hub) {
	t.Helper()
	ln, err := hub.Listen(echoPort)
	if err != nil {
		t.Fatalf("Listen: %v", err)
	}
	t.Cleanup(func() { ln.Close() })
	go func() {
		for {
			c, err := ln.Accept()
			if err != nil {
				return
			}
			go func() { io.Copy(c, c); c.Close() }()
		}
	}()
}

func newDevice(t *testing.T, hub *tunnel.Hub, key tunnel.PrivateKey) *tunnel.Device {
	t.Helper()
	device, err := tunnel.NewDevice(tunnel.DeviceConfig{Key: key, Hub: hub.Address()})
	if err != nil {
		t.Fatalf("NewDevice: %v", err)
	}
	t.Cleanup(func() { device.Close() })
	return device
}

func dial(t *testing.T, device *tunnel.Device) net.Conn {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), 20*time.Second)
	defer cancel()
	c, err := device.Dial(ctx, echoPort)
	if err != nil {
		t.Fatalf("Dial: %v", err)
	}
	t.Cleanup(func() { c.Close() })
	return c
}

func echo(t *testing.T, c net.Conn, message string) {
	t.Helper()
	c.SetDeadline(time.Now().Add(10 * time.Second))
	if _, err := c.Write([]byte(message)); err != nil {
		t.Fatalf("write: %v", err)
	}
	got := make([]byte, len(message))
	if _, err := io.ReadFull(c, got); err != nil {
		t.Fatalf("read: %v", err)
	}
	if !bytes.Equal(got, []byte(message)) {
		t.Fatalf("echo = %q, want %q", got, message)
	}
}

func admitAll(tunnel.DeviceKey) bool { return true }

// admission is an Admit answer that a test can change while the hub runs.
type admission struct{ atomic.Bool }

func (a *admission) admit(tunnel.DeviceKey) bool { return a.Load() }

// waitForPath pings until the device reports the wanted kind of path.
func waitForPath(t *testing.T, device *tunnel.Device, want tunnel.PathKind) tunnel.Path {
	t.Helper()
	deadline := time.Now().Add(30 * time.Second)
	for {
		ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
		path, err := device.Path(ctx)
		cancel()
		if err == nil && path.Kind == want {
			return path
		}
		if time.Now().After(deadline) {
			t.Fatalf("path = %+v (%v), want %s", path, err, want)
		}
		time.Sleep(200 * time.Millisecond)
	}
}

func TestBytesCrossTheLinkOnADirectPath(t *testing.T) {
	hub := startHub(t, newIdentity(t), admitAll)
	serveEcho(t, hub)
	key := tunnel.NewPrivateKey()
	device := newDevice(t, hub, key)

	echo(t, dial(t, device), "through the link")

	waitForPath(t, device, tunnel.PathDirect)
	if got := hub.Paths()[key.Public()].Kind; got != tunnel.PathDirect {
		t.Errorf("hub reports %q for the device, want direct", got)
	}
}

func TestBytesCrossTheLinkThroughTheRelay(t *testing.T) {
	testbed.ForceRelay(t)
	id := newIdentity(t)
	hub := startHub(t, id, admitAll)
	serveEcho(t, hub)
	key := tunnel.NewPrivateKey()
	device := newDevice(t, hub, key)

	echo(t, dial(t, device), "through the relay")

	if path := waitForPath(t, device, tunnel.PathRelayed); path.Latency <= 0 {
		t.Errorf("relayed path has no latency: %+v", path)
	}
	got := hub.Paths()[key.Public()]
	if got.Kind != tunnel.PathRelayed || got.Relay != id.relay.Name() || got.Relay == "" {
		t.Errorf("hub reports %+v for the device, want relayed through %q", got, id.relay.Name())
	}
}

func TestADeviceNeedsAValidAddressAndAKey(t *testing.T) {
	if _, err := tunnel.ParseAddress("not an address"); err != tunnel.ErrBadAddress {
		t.Errorf("bad address text: err = %v, want ErrBadAddress", err)
	}
	if _, err := tunnel.NewDevice(tunnel.DeviceConfig{Key: tunnel.NewPrivateKey()}); err != tunnel.ErrBadAddress {
		t.Errorf("no address: err = %v, want ErrBadAddress", err)
	}
	if _, err := tunnel.NewDevice(tunnel.DeviceConfig{}); err != tunnel.ErrNoIdentity {
		t.Errorf("no key: err = %v, want ErrNoIdentity", err)
	}
}

func TestAHubNeedsAnAdmitFunctionAndItsKeys(t *testing.T) {
	ctx := context.Background()
	if _, err := tunnel.StartHub(ctx, tunnel.HubConfig{Key: tunnel.NewPrivateKey(), PresharedKey: tunnel.NewPresharedKey()}); err != tunnel.ErrNoAdmit {
		t.Errorf("no Admit: err = %v, want ErrNoAdmit", err)
	}
	if _, err := tunnel.StartHub(ctx, tunnel.HubConfig{Admit: admitAll}); err != tunnel.ErrNoIdentity {
		t.Errorf("no keys: err = %v, want ErrNoIdentity", err)
	}
}
