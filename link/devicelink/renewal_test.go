package devicelink_test

import (
	"bufio"
	"net"
	"net/http"
	"sync"
	"testing"
	"time"

	"github.com/TheArctesian/anima/link/devicelink"
	"github.com/TheArctesian/anima/link/identity"
	"github.com/TheArctesian/anima/link/tunnel"
)

const days = 24 * time.Hour

// kept is where a device link saves its identity in these tests.
type kept struct {
	mu     sync.Mutex
	device identity.Device
	// saves is each identity that was saved, in order.
	saves []identity.Device
}

func (k *kept) save(device identity.Device) error {
	k.mu.Lock()
	defer k.mu.Unlock()
	k.device = device
	k.saves = append(k.saves, device)
	return nil
}

func (k *kept) history() []identity.Device {
	k.mu.Lock()
	defer k.mu.Unlock()
	return append([]identity.Device(nil), k.saves...)
}

// waitForSavedKey waits until the device has saved the wanted key as its key.
func (k *kept) waitForSavedKey(t *testing.T, want tunnel.DeviceKey) {
	t.Helper()
	for deadline := time.Now().Add(30 * time.Second); k.get().Key.Public() != want; time.Sleep(50 * time.Millisecond) {
		if time.Now().After(deadline) {
			t.Fatal("the device did not save its next key as its key")
		}
	}
}

func (k *kept) get() identity.Device {
	k.mu.Lock()
	defer k.mu.Unlock()
	return k.device
}

// joined gives a device that has joined the hub with an invite.
func (h invitingHub) joined(t *testing.T) identity.Device {
	t.Helper()
	device, err := join(h.invite(t, "laptop"), identity.Device{Key: tunnel.NewPrivateKey()}, 30*time.Second)
	if err != nil {
		t.Fatal(err)
	}
	return device
}

// startRenewing starts a device link that renews its key by the hub's clock.
func (h invitingHub) startRenewing(t *testing.T, device identity.Device) (*devicelink.Link, *kept) {
	t.Helper()
	saved := &kept{device: device}
	link, err := devicelink.Start(devicelink.Config{
		Identity: device, Listen: "127.0.0.1:0", Keepalive: 300 * time.Millisecond, Save: saved.save, Now: h.clock.Now,
	})
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { link.Close() })
	return link, saved
}

// waitForKey waits until the hub's list holds the one device with the wanted key.
func (h invitingHub) waitForKey(t *testing.T, want tunnel.DeviceKey, why string) {
	t.Helper()
	deadline := time.Now().Add(30 * time.Second)
	for {
		all := h.devices.All()
		if len(all) == 1 && all[0].Key == want && all[0].NextKey.IsZero() {
			return
		}
		if time.Now().After(deadline) {
			t.Fatalf("%s: the hub's list is %+v", why, all)
		}
		time.Sleep(100 * time.Millisecond)
	}
}

func (h invitingHub) refuses(t *testing.T, old identity.Device) {
	t.Helper()
	time.Sleep(time.Second) // the tunnel forgets a replaced key half a second after the change
	link, err := devicelink.Start(devicelink.Config{Identity: old, Listen: "127.0.0.1:0", Keepalive: time.Second})
	if err != nil {
		t.Fatal(err)
	}
	defer link.Close()
	if got, err := get(link, 4*time.Second); err == nil {
		t.Errorf("the old key still reaches the hub: %q", got)
	}
}

func TestLNK005ADeviceRenewsItsKeyWithNoUserAction(t *testing.T) {
	hub := startInvitingHub(t)
	device := hub.joined(t)
	if !device.Expires.Equal(hub.clock.Now().Add(180 * days)) {
		t.Fatalf("the device learned the expiry %v at the join, want 180 days from now", device.Expires)
	}
	device.HubName = "hub.example.org"
	link, saved := hub.startRenewing(t, device)
	getUntil(t, link, 30*time.Second)

	hub.clock.advance(149 * days)
	time.Sleep(2 * time.Second) // several keepalives
	if hub.devices.All()[0].Key != device.Key.Public() || !saved.get().NextKey.IsZero() {
		t.Fatal("the device renewed its key 31 days before the expiry")
	}

	hub.clock.advance(2 * days)
	deadline := time.Now().Add(30 * time.Second)
	for saved.get().Key.Public() == device.Key.Public() {
		if time.Now().After(deadline) {
			t.Fatal("the device did not renew its key in its last 30 days")
		}
		time.Sleep(100 * time.Millisecond)
	}
	renewed := saved.get()
	hub.waitForKey(t, renewed.Key.Public(), "after the renewal")
	if renewed.HubName != device.HubName {
		t.Error("the renewal lost the hub's domain name")
	}
	if !renewed.NextKey.IsZero() || !renewed.Expires.Equal(hub.clock.Now().Add(180*days)) || renewed.Hub != device.Hub {
		t.Errorf("saved identity after the renewal: next key set: %v, expires %v", !renewed.NextKey.IsZero(), renewed.Expires)
	}
	// Step 1 comes first: the next key is on disk, beside the key that still
	// works, before the hub is told. A device that stops there has both.
	first := saved.history()[0]
	if first.Key.Public() != device.Key.Public() || first.NextKey.IsZero() || first.NextKey.Public() != renewed.Key.Public() {
		t.Error("the first save of the renewal must hold the old key as the key and the next key beside it")
	}
	getUntil(t, link, 30*time.Second)
	hub.refuses(t, device)
}

func TestLNK005ARenewalThatWasCutLeavesOneWorkingKeyAndCompletes(t *testing.T) {
	for name, hubWasTold := range map[string]bool{
		"the device stopped before it told the hub":     false,
		"the device stopped after the hub had accepted": true,
	} {
		t.Run(name, func(t *testing.T) {
			hub := startInvitingHub(t)
			device := hub.joined(t)
			hub.clock.advance(160 * days)
			device.NextKey = tunnel.NewPrivateKey()
			if hubWasTold {
				if _, err := hub.devices.Renew(device.Key.Public(), device.NextKey.Public()); err != nil {
					t.Fatal(err)
				}
			}

			link, saved := hub.startRenewing(t, device)
			getUntil(t, link, 30*time.Second)
			hub.waitForKey(t, device.NextKey.Public(), "after the device came back")
			saved.waitForSavedKey(t, device.NextKey.Public())
			getUntil(t, link, 30*time.Second)
		})
	}
}

func TestLNK005AnOpenSessionEndsWhenItsKeyExpires(t *testing.T) {
	hub := startInvitingHub(t)
	device := hub.joined(t)
	link, err := devicelink.Start(devicelink.Config{Identity: device, Listen: "127.0.0.1:0", Keepalive: 300 * time.Millisecond})
	if err != nil {
		t.Fatal(err)
	}
	defer link.Close()
	getUntil(t, link, 30*time.Second)

	// One connection that stays open after its first request, as a session does.
	session, err := net.Dial("tcp", link.LocalAddr().String())
	if err != nil {
		t.Fatal(err)
	}
	defer session.Close()
	request, _ := http.NewRequest("GET", "http://hub/", nil)
	if err := request.Write(session); err != nil {
		t.Fatal(err)
	}
	reader := bufio.NewReader(session)
	response, err := http.ReadResponse(reader, request)
	if err != nil {
		t.Fatal(err)
	}
	response.Body.Close()

	hub.clock.advance(181 * days)

	session.SetReadDeadline(time.Now().Add(10 * time.Second))
	if _, err := reader.ReadByte(); err == nil {
		t.Fatal("the session got data after the key expired")
	} else if timeout, ok := err.(net.Error); ok && timeout.Timeout() {
		t.Fatal("the open session is still open 10 s after its key expired (sweep: 200 ms)")
	}
	if got, err := get(link, 4*time.Second); err == nil {
		t.Errorf("a device with an expired key reached the hub: %q", got)
	}
}
