package devicelink_test

import (
	"context"
	"errors"
	"fmt"
	"io"
	"net/http"
	"net/http/httptest"
	"os"
	"testing"
	"time"

	"github.com/TheArctesian/anima/link/devicelink"
	"github.com/TheArctesian/anima/link/hublink"
	"github.com/TheArctesian/anima/link/identity"
	"github.com/TheArctesian/anima/link/tunnel"
	"github.com/TheArctesian/anima/link/tunnel/testbed"
)

func TestMain(m *testing.M) { testbed.Main(m) }

const greeting = "the hub answers"

// site is one hub with one permitted device: what the hub keeps between
// starts, and the device's identity.
type site struct {
	hub    identity.Hub
	target string
	device identity.Device
}

func newSite(t *testing.T) *site {
	t.Helper()
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {
		fmt.Fprint(w, greeting)
	}))
	t.Cleanup(server.Close)
	return &site{
		hub:    identity.Hub{Key: tunnel.NewPrivateKey(), PresharedKey: tunnel.NewPresharedKey(), Relay: testbed.StartRelay(t)},
		target: server.Listener.Addr().String(),
		device: identity.Device{Key: tunnel.NewPrivateKey()},
	}
}

// startHub starts the hub link. It also gives the device the hub address,
// as an enrolment does.
func (s *site) startHub(t *testing.T) *hublink.Link {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), 20*time.Second)
	defer cancel()
	link, err := hublink.Start(ctx, hublink.Config{
		Identity: s.hub,
		Target:   s.target,
		Permits:  func(key tunnel.DeviceKey) bool { return key == s.device.Key.Public() },
	})
	if err != nil {
		t.Fatalf("hublink.Start: %v", err)
	}
	t.Cleanup(func() { link.Close() })
	s.device.Hub = link.Address()
	return link
}

func (s *site) startDevice(t *testing.T, keepalive time.Duration) *devicelink.Link {
	t.Helper()
	link, err := devicelink.Start(devicelink.Config{Identity: s.device, Listen: "127.0.0.1:0", Keepalive: keepalive})
	if err != nil {
		t.Fatalf("devicelink.Start: %v", err)
	}
	t.Cleanup(func() { link.Close() })
	return link
}

// get makes one request through the local port, on a new connection.
func get(link *devicelink.Link, limit time.Duration) (string, error) {
	client := &http.Client{Timeout: limit, Transport: &http.Transport{DisableKeepAlives: true}}
	response, err := client.Get("http://" + link.LocalAddr().String())
	if err != nil {
		return "", err
	}
	defer response.Body.Close()
	body, err := io.ReadAll(response.Body)
	return string(body), err
}

// getUntil repeats a request until the hub answers, and returns how long it took.
func getUntil(t *testing.T, link *devicelink.Link, limit time.Duration) time.Duration {
	t.Helper()
	start := time.Now()
	for {
		got, err := get(link, 5*time.Second)
		if err == nil && got == greeting {
			return time.Since(start)
		}
		if time.Since(start) > limit {
			t.Fatalf("no answer from the hub through the local port in %v (last: %q, %v)", limit, got, err)
		}
		time.Sleep(250 * time.Millisecond)
	}
}

func TestTheLocalPortReachesTheHub(t *testing.T) {
	s := newSite(t)
	s.startHub(t)
	device := s.startDevice(t, time.Second)

	start := time.Now()
	got, err := get(device, 30*time.Second)
	if err != nil || got != greeting {
		t.Fatalf("through the local port: %q, %v", got, err)
	}
	t.Logf("first request of a new device link: %v", time.Since(start).Round(time.Millisecond))

	deadline := time.Now().Add(30 * time.Second)
	for device.Path().Kind != tunnel.PathDirect {
		if time.Now().After(deadline) {
			t.Fatalf("path = %+v, want direct on one machine", device.Path())
		}
		time.Sleep(100 * time.Millisecond)
	}
}

func TestLNK009ARunningDeviceLinkReachesTheHubAgainAfterAHubRestart(t *testing.T) {
	s := newSite(t)
	hub := s.startHub(t)
	device := s.startDevice(t, time.Second)
	getUntil(t, device, 30*time.Second)

	hub.Close()
	if got, err := get(device, 3*time.Second); err == nil {
		t.Fatalf("the hub link is stopped, but a request got %q", got)
	}
	s.startHub(t)

	took := getUntil(t, device, 45*time.Second)
	t.Logf("the device link reached the restarted hub after %v, with no restart of the device link", took.Round(time.Millisecond))
}

func TestADeviceLinkStartsBeforeItsHubAndReachesItLater(t *testing.T) {
	s := newSite(t)
	first := s.startHub(t) // gives the device the address
	first.Close()
	device := s.startDevice(t, time.Second)
	if device.Path().Kind != tunnel.PathNone {
		t.Errorf("path with no hub = %+v, want none", device.Path())
	}

	s.startHub(t)
	getUntil(t, device, 45*time.Second)
}

func TestTheLocalPortMustBeOnLoopbackAndTheDeviceEnrolled(t *testing.T) {
	s := newSite(t)
	s.startHub(t)
	for _, listen := range []string{"0.0.0.0:0", "192.0.2.1:8080", "localhost:0", ""} {
		_, err := devicelink.Start(devicelink.Config{Identity: s.device, Listen: listen})
		if !errors.Is(err, devicelink.ErrListen) {
			t.Errorf("listen %q: err = %v, want ErrListen", listen, err)
		}
	}
	_, err := devicelink.Start(devicelink.Config{Identity: identity.Device{Key: tunnel.NewPrivateKey()}, Listen: "127.0.0.1:0"})
	if !errors.Is(err, devicelink.ErrNotEnrolled) {
		t.Errorf("no hub address: err = %v, want ErrNotEnrolled", err)
	}
}

// TestLNK009AnIdleLinkDoesNotStall takes more than 4 minutes of real time,
// because the stall that it guards against comes from the timers of the
// tunnel. It runs only when ANIMA_LINK_SLOW_TESTS is set.
func TestLNK009AnIdleLinkDoesNotStall(t *testing.T) {
	if os.Getenv("ANIMA_LINK_SLOW_TESTS") == "" {
		t.Skip("set ANIMA_LINK_SLOW_TESTS=1 to run this 4-minute test")
	}
	s := newSite(t)
	s.startHub(t)
	device := s.startDevice(t, devicelink.DefaultKeepalive)
	getUntil(t, device, 30*time.Second)

	time.Sleep(4 * time.Minute)

	start := time.Now()
	if got, err := get(device, 30*time.Second); err != nil || got != greeting {
		t.Fatalf("after 4 minutes idle: %q, %v", got, err)
	}
	if took := time.Since(start); took > time.Second {
		t.Errorf("the first request after 4 minutes idle took %v, want under 1 s", took)
	} else {
		t.Logf("the first request after 4 minutes idle took %v", took.Round(time.Millisecond))
	}
}
