package main

import (
	"context"
	"io"
	"net"
	"net/http"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"

	"github.com/TheArctesian/anima/link/hublink/testhub"
)

// TestK8TheDaemonReachesTheHubThroughTheSocketOfTheDeviceLink is the link
// half of K8 Section 1: a request on the Unix socket of "-daemon-socket"
// reaches the hub, as a request on the local port does.
func TestK8TheDaemonReachesTheHubThroughTheSocketOfTheDeviceLink(t *testing.T) {
	socket := testhub.Start(t)
	state := stateDir(t)
	seedRelay(t, state)
	hub := startHubInFrontOf(t, state, socket.Socket)
	folder := filepath.Dir(stateDir(t))
	if err := os.Chmod(folder, 0o700); err != nil {
		t.Fatal(err)
	}
	forDaemon := filepath.Join(folder, "link.sock")
	startDevice(t, hub, "-daemon-socket", forDaemon)

	client := &http.Client{Timeout: 5 * time.Second, Transport: &http.Transport{
		DialContext: func(ctx context.Context, _, _ string) (net.Conn, error) {
			return (&net.Dialer{}).DialContext(ctx, "unix", forDaemon)
		},
	}}
	deadline := time.Now().Add(30 * time.Second)
	for {
		response, err := client.Get("http://127.0.0.1/")
		if err == nil {
			seen, _ := io.ReadAll(response.Body)
			response.Body.Close()
			if !strings.Contains(string(seen), ":") {
				t.Errorf("the hub saw the device as %q, want its link address", seen)
			}
			return
		}
		if time.Now().After(deadline) {
			t.Fatalf("the hub did not answer through the socket of the daemon: %v", err)
		}
		time.Sleep(250 * time.Millisecond)
	}
}

func TestK8ADeviceLinkDoesNotStartWithADaemonSocketThatOtherUsersCanReach(t *testing.T) {
	folder := filepath.Dir(stateDir(t))
	if err := os.Chmod(folder, 0o755); err != nil {
		t.Fatal(err)
	}
	log := &output{}
	code := run(context.Background(), []string{"device", "-state", stateDir(t), "-daemon-socket", filepath.Join(folder, "link.sock")}, stdio{strings.NewReader(""), io.Discard, log})
	if code == 0 || !strings.Contains(log.String(), "other users can reach") {
		t.Fatalf("exit %d, log %q; want a refusal that names the exposed path", code, log)
	}
}
