package devicelink_test

import (
	"testing"
	"time"

	"github.com/TheArctesian/anima/link/devicelink"
	"github.com/TheArctesian/anima/link/tunnel/testbed"
)

func TestLNK007AConnectedDeviceFollowsTheHubToANewRelayWithNoNewInvite(t *testing.T) {
	hub := startInvitingHub(t)
	device := hub.joined(t)
	absent := hub.joined(t) // a second device, which is off during the change
	saved := &kept{device: device}
	link, err := devicelink.Start(devicelink.Config{
		Identity: device, Listen: "127.0.0.1:0", Keepalive: 300 * time.Millisecond, NoticeEvery: 1, Save: saved.save,
	})
	if err != nil {
		t.Fatal(err)
	}
	defer link.Close()
	getUntil(t, link, 30*time.Second)

	// Step 1: the hub announces the address that it will have.
	newRelay := testbed.StartRelay(t)
	announced := hub.link.Announce(newRelay)
	if announced == hub.link.Address() {
		t.Fatal("the address on the new relay is the same as the address now")
	}
	for deadline := time.Now().Add(30 * time.Second); saved.get().NextHub != announced; time.Sleep(100 * time.Millisecond) {
		if time.Now().After(deadline) {
			t.Fatal("the connected device did not learn the hub's next address")
		}
	}
	if saved.get().Hub != device.Hub {
		t.Fatal("the device changed its hub address before the hub moved")
	}

	// Step 2: the hub link starts again on the new relay.
	hub.link.Close()
	hub.identity.Relay = newRelay
	moved := hub.startLink(t)
	if moved.Address() != announced {
		t.Fatal("the hub's address on the new relay is not the address that it announced")
	}

	took := getUntil(t, link, 60*time.Second)
	t.Logf("the device reached the hub on its new relay after %v", took.Round(time.Millisecond))
	if after := saved.get(); after.Hub != announced || !after.NextHub.IsZero() {
		t.Error("the device did not save the new address as the hub's address")
	}
	if hub.devices.Count() != 2 {
		t.Errorf("the relay change changed the list of devices: %d", hub.devices.Count())
	}

	// The device that was off still has the old address only.
	old, err := devicelink.Start(devicelink.Config{Identity: absent, Listen: "127.0.0.1:0", Keepalive: 300 * time.Millisecond, Save: (&kept{}).save})
	if err != nil {
		t.Fatal(err)
	}
	defer old.Close()
	if got, err := get(old, 5*time.Second); err == nil {
		t.Errorf("a device that was off during the change reached the hub at its old address: %q", got)
	}
}

func TestLNK007ADeviceReadsNoNextAddressWhenNoChangeIsPlanned(t *testing.T) {
	hub := startInvitingHub(t)
	device := hub.joined(t)
	saved := &kept{device: device}
	link, err := devicelink.Start(devicelink.Config{
		Identity: device, Listen: "127.0.0.1:0", Keepalive: 200 * time.Millisecond, NoticeEvery: 1, Save: saved.save,
	})
	if err != nil {
		t.Fatal(err)
	}
	defer link.Close()
	getUntil(t, link, 30*time.Second)
	time.Sleep(time.Second)
	if !saved.get().NextHub.IsZero() {
		t.Error("the device has a next address although the hub announced none")
	}
}
