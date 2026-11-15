package devicelink_test

import (
	"context"
	"errors"
	"sync"
	"testing"
	"time"

	"github.com/TheArctesian/anima/link/devicelink"
	"github.com/TheArctesian/anima/link/devices"
	"github.com/TheArctesian/anima/link/identity"
	"github.com/TheArctesian/anima/link/tunnel"
	"github.com/TheArctesian/anima/link/tunnel/testbed"
)

// zone is a resolver that a test fills.
type zone struct {
	mu      sync.Mutex
	records map[string][]string
	asked   []string
}

func (z *zone) lookup(_ context.Context, name string) ([]string, error) {
	z.mu.Lock()
	defer z.mu.Unlock()
	z.asked = append(z.asked, name)
	if records, found := z.records[name]; found {
		return records, nil
	}
	return nil, errors.New("no such name")
}

func (z *zone) names() []string {
	z.mu.Lock()
	defer z.mu.Unlock()
	return append([]string(nil), z.asked...)
}

func (z *zone) set(name string, records ...string) {
	z.mu.Lock()
	defer z.mu.Unlock()
	if z.records == nil {
		z.records = map[string][]string{}
	}
	z.records[name] = records
}

// A device that was off while the hub changed its relay has only the old
// address. With the hub's domain name it finds the hub again.
func TestLNK007ADeviceFindsAHubThatMovedByItsDomainName(t *testing.T) {
	hub := startInvitingHub(t)
	device := hub.joined(t)
	device.HubName = "hub.example.org"
	stranger := identity.Device{Key: tunnel.NewPrivateKey(), Hub: device.Hub, HubName: device.HubName}

	hub.link.Close()
	hub.identity.Relay = testbed.StartRelay(t)
	moved := hub.startLink(t)
	names := &zone{}
	names.set("_anima-link.hub.example.org", "v=spf1 -all", moved.Address().Reveal())

	start := func(who identity.Device) (*devicelink.Link, *kept) {
		saved := &kept{device: who}
		link, err := devicelink.Start(devicelink.Config{
			Identity: who, Listen: "127.0.0.1:0", Keepalive: 300 * time.Millisecond, AnswerLimit: time.Second, Save: saved.save, LookupTXT: names.lookup,
		})
		if err != nil {
			t.Fatal(err)
		}
		t.Cleanup(func() { link.Close() })
		return link, saved
	}

	// The stranger starts at the same time, so it has as long as the device
	// to read the name and to try.
	strangerLink, _ := start(stranger)
	link, saved := start(device)
	getUntil(t, link, 90*time.Second)
	if saved.get().Hub != moved.Address() {
		t.Error("the device did not save the address from the domain name as the hub's address")
	}

	// A name only finds the hub. A key that is not in the list is still refused.
	if got, err := get(strangerLink, 4*time.Second); err == nil {
		t.Errorf("a device that is not permitted reached the hub through its domain name: %q", got)
	}
}

// DNS can be answered by anybody. An address in it with another hub's keys
// is another hub, and the device must not move to it.
func TestLNK007ADeviceDoesNotFollowADomainNameToAnotherHub(t *testing.T) {
	hub := startInvitingHub(t)
	device := hub.joined(t)
	device.HubName = "hub.example.org"
	hub.link.Close() // the real hub is off, so the device looks for it

	impostor := startInvitingHub(t)
	if _, err := impostor.devices.Permit(devices.Device{Key: device.Key.Public(), Name: "the victim"}); err != nil {
		t.Fatal(err)
	}
	names := &zone{}
	names.set("_anima-link.hub.example.org", impostor.link.Address().Reveal())
	saved := &kept{device: device}
	link, err := devicelink.Start(devicelink.Config{
		Identity: device, Listen: "127.0.0.1:0", Keepalive: 300 * time.Millisecond, AnswerLimit: time.Second, Save: saved.save, LookupTXT: names.lookup,
	})
	if err != nil {
		t.Fatal(err)
	}
	defer link.Close()

	for deadline := time.Now().Add(90 * time.Second); len(names.names()) == 0; time.Sleep(200 * time.Millisecond) {
		if time.Now().After(deadline) {
			t.Fatal("the device never asked the domain name; the test proves nothing")
		}
	}
	time.Sleep(5 * time.Second) // longer than the time to register at an address (2 s here)
	if saved.get().Hub != device.Hub {
		t.Fatal("the device saved the address of another hub as its hub")
	}
	if got, err := get(link, 4*time.Second); err == nil {
		t.Errorf("the device's local port reaches another hub: %q", got)
	}
}
