package hublink_test

import (
	"testing"
	"time"

	"github.com/TheArctesian/anima/link/tunnel"
	"github.com/TheArctesian/anima/link/tunnel/testbed"
)

// waitForPath waits until the hub link reports the wanted kind of path for a device.
func (r rig) waitForPath(t *testing.T, device tunnel.DeviceKey, want tunnel.PathKind) tunnel.Path {
	t.Helper()
	deadline := time.Now().Add(30 * time.Second)
	for {
		path := r.link.Path(device)
		if path.Kind == want {
			return path
		}
		if time.Now().After(deadline) {
			t.Fatalf("the hub link reports %+v for the device, want %s", path, want)
		}
		time.Sleep(100 * time.Millisecond)
	}
}

func TestLNK006TheHubLinkReportsADirectPath(t *testing.T) {
	r := start(t)
	key, never := tunnel.NewPrivateKey(), tunnel.NewPrivateKey()
	r.permit(t, key)
	r.permit(t, never)
	session := r.session(t, key)

	if path := r.waitForPath(t, key.Public(), tunnel.PathDirect); path.Relay != "" {
		t.Errorf("a direct path names a relay: %+v", path)
	}
	if path := r.link.Path(never.Public()); path.Kind != tunnel.PathNone {
		t.Errorf("a device that never connected has the path %+v, want none", path)
	}
	echoOnce(t, session)
}

func TestLNK006TheHubLinkReportsARelayedPathWithTheRelaysName(t *testing.T) {
	testbed.ForceRelay(t)
	r := start(t)
	key := tunnel.NewPrivateKey()
	r.permit(t, key)
	r.session(t, key)

	path := r.waitForPath(t, key.Public(), tunnel.PathRelayed)
	if path.Relay == "" || path.Relay != r.link.Relay().Name() {
		t.Errorf("relayed path = %+v, want the name of the hub's relay (%q)", path, r.link.Relay().Name())
	}
}

func TestLNK006ARevokedDeviceHasNoPath(t *testing.T) {
	r := start(t)
	key := tunnel.NewPrivateKey()
	r.permit(t, key)
	r.session(t, key)
	r.waitForPath(t, key.Public(), tunnel.PathDirect)

	if _, err := r.link.Revoke(key.Public()); err != nil {
		t.Fatal(err)
	}
	r.waitForPath(t, key.Public(), tunnel.PathNone)
}
