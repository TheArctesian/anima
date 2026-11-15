package tunnel_test

import (
	"context"
	"errors"
	"testing"
	"time"

	"github.com/TheArctesian/anima/link/tunnel"
	"github.com/TheArctesian/anima/link/tunnel/testbed"
)

func TestLNK007ARelayThatRefusesTheHubIsAnErrorAtStart(t *testing.T) {
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()
	began := time.Now()
	_, err := tunnel.StartHub(ctx, tunnel.HubConfig{
		Key: tunnel.NewPrivateKey(), PresharedKey: tunnel.NewPresharedKey(),
		Relay: testbed.StartRefusingRelay(t), Admit: admitAll,
	})
	if !errors.Is(err, tunnel.ErrRelay) {
		t.Fatalf("a hub on a relay that refuses it: err = %v after %v, want ErrRelay", err, time.Since(began).Round(time.Second))
	}
}

func TestLNK007TheRelayCheckLeavesNoDeviceBehind(t *testing.T) {
	hub := startHub(t, newIdentity(t), func(tunnel.DeviceKey) bool { return false })
	for device, path := range hub.Paths() {
		if path.Kind != tunnel.PathNone {
			t.Errorf("after the start a key is connected that no list holds: %s", device)
		}
	}
}

func TestLNK007AnOwnRelayIsAHostNameWithAnOptionalPort(t *testing.T) {
	for _, good := range []string{"relay.example.org", "relay.example.org:8443", "192.0.2.7:443", "relay-1"} {
		relay, err := tunnel.OwnRelay(good)
		if err != nil || relay.IsZero() || relay.Name() == "" {
			t.Errorf("OwnRelay(%q) = %v, want a relay with a name", good, err)
		}
	}
	for _, bad := range []string{"", ":443", "relay.example.org:0", "relay.example.org:port", "bad host", "host/path", "a\x1bb"} {
		if _, err := tunnel.OwnRelay(bad); !errors.Is(err, tunnel.ErrBadRelay) {
			t.Errorf("OwnRelay(%q) = %v, want ErrBadRelay", bad, err)
		}
	}
}
