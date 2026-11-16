package main

import (
	"encoding/json"
	"io"
	"net/http"
	"testing"
	"time"

	"github.com/TheArctesian/anima/link/hublink/testhub"
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
