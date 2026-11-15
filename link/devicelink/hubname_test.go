package devicelink_test

import (
	"context"
	"errors"
	"sync"
	"testing"
	"time"

	"github.com/TheArctesian/anima/link/devicelink"
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
			Identity: who, Listen: "127.0.0.1:0", Keepalive: 300 * time.Millisecond, Save: saved.save, LookupTXT: names.lookup,
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
