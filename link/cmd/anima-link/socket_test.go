package main

import (
	"encoding/json"
	"io"
	"net/http"
	"testing"
	"time"

	"github.com/TheArctesian/anima/link/hublink/testhub"
	"github.com/TheArctesian/anima/link/tunnel"
)

func TestLNK002AHubOnItsSocketSeesADeviceByTheAddressThatTheListShows(t *testing.T) {
	socket := testhub.Start(t)
	state := stateDir(t)
	seedRelay(t, state)
	hub := startHubInFrontOf(t, state, socket.Socket)
	_, local := startDevice(t, hub)

	text, code := hub.ctl(t, "devices.list")
	var listed []struct{ Name, Address string }
	if err := json.Unmarshal([]byte(text), &listed); code != 0 || err != nil || len(listed) != 1 || listed[0].Address == "" {
		t.Fatalf("devices.list = %q, exit %d; want one device with its address", text, code)
	}

	client := &http.Client{Timeout: 5 * time.Second}
	deadline := time.Now().Add(30 * time.Second)
	for {
		response, err := client.Get("http://" + local)
		if err == nil {
			seen, _ := io.ReadAll(response.Body)
			response.Body.Close()
			if string(seen) != listed[0].Address {
				t.Errorf("the hub saw the device as %q, want the listed address %q", seen, listed[0].Address)
			}
			return
		}
		if time.Now().After(deadline) {
			t.Fatalf("the hub did not answer through the device's local port: %v", err)
		}
		time.Sleep(250 * time.Millisecond)
	}
}

// seenOf asks the hub link for its devices and returns when each was last
// seen, by its name. A device that was never seen has nil.
func seenOf(t *testing.T, hub runningHub) map[string]*time.Time {
	t.Helper()
	text, code := hub.ctl(t, "devices.list")
	var listed []struct {
		Name string
		Seen *time.Time
	}
	if err := json.Unmarshal([]byte(text), &listed); code != 0 || err != nil {
		t.Fatalf("devices.list = %q, exit %d", text, code)
	}
	seen := map[string]*time.Time{}
	for _, device := range listed {
		seen[device.Name] = device.Seen
	}
	return seen
}

func TestHMD047TheDeviceListShowsWhenADeviceProgramWasLastSeen(t *testing.T) {
	hub := startHub(t)
	if text, code := hub.permit(t, tunnel.NewPrivateKey(), "off"); code != 0 {
		t.Fatalf("devices.permit: %q, exit %d", text, code)
	}
	before := time.Now().Add(-time.Second)
	startDevice(t, hub)

	deadline := time.Now().Add(30 * time.Second)
	seen := seenOf(t, hub)
	for seen["device link"] == nil {
		if time.Now().After(deadline) {
			t.Fatal("devices.list shows no time for the running device after 30 s")
		}
		time.Sleep(250 * time.Millisecond)
		seen = seenOf(t, hub)
	}
	if at := *seen["device link"]; at.Before(before) || at.After(time.Now().Add(time.Second)) {
		t.Errorf("seen = %v, want the time of this test", at)
	}
	if at, listed := seen["off"]; !listed || at != nil {
		t.Errorf("the device that never connected: listed = %v, seen = %v; want listed with no time", listed, at)
	}
}
