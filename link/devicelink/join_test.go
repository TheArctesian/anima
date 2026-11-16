package devicelink_test

import (
	"context"
	"crypto/rand"
	"errors"
	"fmt"
	"net/http"
	"net/http/httptest"
	"path/filepath"
	"sync"
	"testing"
	"time"

	"github.com/TheArctesian/anima/link/devicelink"
	"github.com/TheArctesian/anima/link/devices"
	"github.com/TheArctesian/anima/link/hublink"
	"github.com/TheArctesian/anima/link/identity"
	"github.com/TheArctesian/anima/link/invites"
	"github.com/TheArctesian/anima/link/statedir"
	"github.com/TheArctesian/anima/link/tunnel"
	"github.com/TheArctesian/anima/link/tunnel/testbed"
)

// clock is a clock that a test moves.
type clock struct {
	mu  sync.Mutex
	now time.Time
}

func (c *clock) Now() time.Time {
	c.mu.Lock()
	defer c.mu.Unlock()
	return c.now
}

func (c *clock) advance(d time.Duration) {
	c.mu.Lock()
	defer c.mu.Unlock()
	c.now = c.now.Add(d)
}

// invitingHub is a hub link that lets devices join with an invite.
type invitingHub struct {
	link    *hublink.Link
	devices *devices.Store
	invites *invites.Store
	clock   *clock
	// identity and target are what the hub link starts from.
	identity identity.Hub
	target   string
}

func startInvitingHub(t *testing.T) invitingHub {
	t.Helper()
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) { fmt.Fprint(w, greeting) }))
	t.Cleanup(server.Close)
	return startInvitingHubInFrontOf(t, server.Listener.Addr().String())
}

// startInvitingHubInFrontOf starts the hub link in front of a target: a
// loopback address or the path of a hub's socket.
func startInvitingHubInFrontOf(t *testing.T, target string) invitingHub {
	t.Helper()
	dir, err := statedir.Open(filepath.Join(t.TempDir(), "s"))
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { dir.Close() })
	hub := invitingHub{
		clock:    &clock{now: time.Date(2026, 11, 14, 12, 0, 0, 0, time.UTC)},
		identity: identity.Hub{Key: tunnel.NewPrivateKey(), PresharedKey: tunnel.NewPresharedKey(), Relay: testbed.StartRelay(t)},
		target:   target,
	}
	if hub.devices, err = devices.Open(dir, hub.clock.Now); err != nil {
		t.Fatal(err)
	}
	if hub.invites, err = invites.Open(dir); err != nil {
		t.Fatal(err)
	}
	hub.link = hub.startLink(t)
	return hub
}

// startLink starts the hub link from the hub's identity and lists.
func (h invitingHub) startLink(t *testing.T) *hublink.Link {
	t.Helper()
	link, err := hublink.Start(t.Context(), hublink.Config{
		Identity: h.identity,
		Target:   h.target,
		Devices:  h.devices,
		Invites:  h.invites,
		Now:      h.clock.Now,
		Sweep:    200 * time.Millisecond,
	})
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { link.Close() })
	return link
}

func (h invitingHub) invite(t *testing.T, name string) invites.Invite {
	t.Helper()
	invite, _, err := h.invites.Make(h.link.Address(), name, h.clock.Now(), rand.Reader)
	if err != nil {
		t.Fatal(err)
	}
	return invite
}

func join(invite invites.Invite, device identity.Device, limit time.Duration) (identity.Device, error) {
	ctx, cancel := context.WithTimeout(context.Background(), limit)
	defer cancel()
	return devicelink.Join(ctx, invite, device)
}

func TestLNK003ADeviceJoinsWithAnInviteAndThenReachesTheHub(t *testing.T) {
	hub := startInvitingHub(t)
	device := identity.Device{Key: tunnel.NewPrivateKey()}

	joined, err := join(hub.invite(t, "Maya's laptop"), device, 30*time.Second)
	if err != nil {
		t.Fatalf("Join: %v", err)
	}
	if joined.Hub != hub.link.Address() || joined.Key.Public() != device.Key.Public() {
		t.Error("Join did not give the device the hub address, or changed its key")
	}
	all := hub.devices.All()
	if len(all) != 1 || all[0].Key != device.Key.Public() || all[0].Name != "Maya's laptop" {
		t.Errorf("permitted devices = %+v, want the device with the invite's name", all)
	}

	link, err := devicelink.Start(devicelink.Config{Identity: joined, Listen: "127.0.0.1:0", Keepalive: time.Second})
	if err != nil {
		t.Fatal(err)
	}
	defer link.Close()
	getUntil(t, link, 30*time.Second)
}

func TestLNK003AnInviteWorksOnce(t *testing.T) {
	hub := startInvitingHub(t)
	invite := hub.invite(t, "laptop")
	if _, err := join(invite, identity.Device{Key: tunnel.NewPrivateKey()}, 30*time.Second); err != nil {
		t.Fatalf("first use: %v", err)
	}

	_, err := join(invite, identity.Device{Key: tunnel.NewPrivateKey()}, 4*time.Second)
	if !errors.Is(err, devicelink.ErrNoAnswer) {
		t.Errorf("second use: err = %v, want no answer", err)
	}
	if hub.devices.Count() != 1 {
		t.Errorf("%d devices are permitted after two uses of one invite, want 1", hub.devices.Count())
	}
}

func TestLNK003AnInviteIsEndedAlsoWhenItsFirstUseFailsLater(t *testing.T) {
	hub := startInvitingHub(t)
	known := identity.Device{Key: tunnel.NewPrivateKey()}
	if _, err := join(hub.invite(t, "laptop"), known, 30*time.Second); err != nil {
		t.Fatal(err)
	}

	// The hub ends the invite, then refuses the device: its key is in the list already.
	invite := hub.invite(t, "the same key again")
	if _, err := join(invite, known, 30*time.Second); !errors.Is(err, devicelink.ErrRefused) {
		t.Fatalf("a use that the hub refuses: err = %v, want ErrRefused", err)
	}
	if _, err := join(invite, identity.Device{Key: tunnel.NewPrivateKey()}, 4*time.Second); !errors.Is(err, devicelink.ErrNoAnswer) {
		t.Errorf("a second use after a failed first use: err = %v, want no answer", err)
	}
}

func TestLNK003AnExpiredOrWrongInviteGetsNoAnswer(t *testing.T) {
	hub := startInvitingHub(t)
	expired := hub.invite(t, "too late")
	wrong := hub.invite(t, "wrong code")
	wrong.Code[0] ^= 0xFF
	hub.clock.advance(15 * time.Minute)
	fresh := hub.invite(t, "in time")

	for name, invite := range map[string]invites.Invite{"expired": expired, "wrong": wrong} {
		if _, err := join(invite, identity.Device{Key: tunnel.NewPrivateKey()}, 4*time.Second); !errors.Is(err, devicelink.ErrNoAnswer) {
			t.Errorf("%s invite: err = %v, want no answer", name, err)
		}
	}
	if _, err := join(fresh, identity.Device{Key: tunnel.NewPrivateKey()}, 30*time.Second); err != nil {
		t.Errorf("an invite made after the others expired: %v", err)
	}
}

func TestLNK003TheTemporaryKeyOfAnInviteDoesNotReachTheHub(t *testing.T) {
	hub := startInvitingHub(t)
	invite := hub.invite(t, "laptop")

	asInvite := identity.Device{Key: invite.TemporaryKey(), Hub: invite.Hub}
	link, err := devicelink.Start(devicelink.Config{Identity: asInvite, Listen: "127.0.0.1:0", Keepalive: time.Second})
	if err != nil {
		t.Fatal(err)
	}
	defer link.Close()
	if got, err := get(link, 4*time.Second); err == nil {
		t.Fatalf("a device that has only an invite got %q from the hub", got)
	}
	if hub.devices.Count() != 0 {
		t.Error("the attempt put a device in the list")
	}
	if _, err := join(invite, identity.Device{Key: tunnel.NewPrivateKey()}, 30*time.Second); err != nil {
		t.Errorf("the invite must still work for its purpose: %v", err)
	}
}

// Whoever reads a used invite later can make its temporary key. So that key
// must never become a device's key.
func TestLNK003TheTemporaryKeyOfAnInviteCannotBecomeADeviceKey(t *testing.T) {
	hub := startInvitingHub(t)
	invite := hub.invite(t, "laptop")

	asInvite := identity.Device{Key: invite.TemporaryKey()}
	if _, err := join(invite, asInvite, 30*time.Second); !errors.Is(err, devicelink.ErrRefused) {
		t.Fatalf("a join that gives the invite's own key: err = %v, want ErrRefused", err)
	}
	if hub.devices.Count() != 0 {
		t.Fatal("the invite's temporary key is in the list of devices")
	}

	asInvite.Hub = invite.Hub
	link, err := devicelink.Start(devicelink.Config{Identity: asInvite, Listen: "127.0.0.1:0", Keepalive: time.Second})
	if err != nil {
		t.Fatal(err)
	}
	defer link.Close()
	if got, err := get(link, 4*time.Second); err == nil {
		t.Errorf("the text of a used invite still reaches the hub: %q", got)
	}
}
