package main

import (
	"bufio"
	"context"
	"encoding/json"
	"github.com/TheArctesian/anima/link/testpath"
	"io"
	"net"
	"net/http"
	"path/filepath"
	"slices"
	"strings"
	"testing"
	"time"

	"github.com/TheArctesian/anima/link/hublink/testhub"
	"github.com/TheArctesian/anima/link/statedir"
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

// dialWord asks the dial socket of a hub link for a local port of a device
// and returns the answer line.
func dialWord(t *testing.T, hub runningHub, device, port string) string {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	conn, err := statedir.Dial(ctx, hub.state, dialSocket)
	if err != nil {
		t.Fatalf("the hub link has no dial socket: %v", err)
	}
	defer conn.Close()
	_ = conn.SetDeadline(time.Now().Add(20 * time.Second))
	if _, err := io.WriteString(conn, "dial "+device+" "+port+"\n"); err != nil {
		t.Fatal(err)
	}
	word, err := bufio.NewReader(conn).ReadString('\n')
	if err != nil {
		t.Fatalf("the dial socket gave no answer line: %v", err)
	}
	return strings.TrimSuffix(word, "\n")
}

// ports runs the command "ports" as a second process would.
func portsOf(t *testing.T, state string, args ...string) (stdout string, code int) {
	t.Helper()
	var out, errs output
	code = run(context.Background(), append([]string{"ports", "-state", state}, args...), stdio{strings.NewReader(""), &out, &errs})
	return strings.TrimSpace(out.String() + errs.String()), code
}

// connected starts a device link and returns its state directory and its
// link address, once the hub link has seen it.
func connected(t *testing.T, hub runningHub) (state, address string) {
	t.Helper()
	state, _ = startDevice(t, hub)
	deadline := time.Now().Add(30 * time.Second)
	for seenOf(t, hub)["device link"] == nil {
		if time.Now().After(deadline) {
			t.Fatal("the device is not connected after 30 s")
		}
		time.Sleep(250 * time.Millisecond)
	}
	text, _ := hub.ctl(t, "devices.list")
	var listed []struct{ Address string }
	if err := json.Unmarshal([]byte(text), &listed); err != nil || len(listed) != 1 {
		t.Fatalf("devices.list = %q", text)
	}
	return state, listed[0].Address
}

// The two programs and the command "ports": the device gives no port until
// its person turns the port on, and the change needs no restart. The test
// cannot open port 22 itself, so with the port on it accepts "ok" (this
// computer has Remote Login) and "closed" (it has not).
func TestHMD048TheDialSocketReachesAPortOnlyAfterThePersonTurnedItOn(t *testing.T) {
	hub := startHub(t)
	state, device := connected(t, hub)
	wantPorts := func(want string, wantCode int, args ...string) {
		t.Helper()
		if got, code := portsOf(t, state, args...); code != wantCode || (want != "" && got != want) {
			t.Fatalf("ports %v = %q, exit %d; want %q, exit %d", args, got, code, want, wantCode)
		}
	}
	wantWord := func(device, port string, want ...string) {
		t.Helper()
		if word := dialWord(t, hub, device, port); !slices.Contains(want, word) {
			t.Fatalf("the dial socket said %q for port %s, want one of %v", word, port, want)
		}
	}

	wantPorts(`{"open":[]}`, 0)
	wantWord(device, "22", "refused")
	wantPorts("", 1, "on", "80")
	wantPorts(`{"open":[22]}`, 0, "on", "22")
	wantWord(device, "22", "ok", "closed")
	wantWord(device, "5900", "refused")
	wantPorts(`{"open":[]}`, 0, "off", "22")
	wantWord(device, "22", "refused")
	wantWord("fd41:4e49:4d41::ffff", "22", "not_connected")
	wantPorts("", 2, "sideways", "22")
}

// TestK8TheDaemonReachesTheHubThroughTheSocketOfTheDeviceLink is the link
// half of K8 Section 1: a request on the Unix socket of "-daemon-socket"
// reaches the hub, as a request on the local port does.
func TestK8TheDaemonReachesTheHubThroughTheSocketOfTheDeviceLink(t *testing.T) {
	socket := testhub.Start(t)
	state := stateDir(t)
	seedRelay(t, state)
	hub := startHubInFrontOf(t, state, socket.Socket)
	folder := filepath.Dir(stateDir(t))
	if err := testpath.Chmod(folder, 0o700); err != nil {
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
	if err := testpath.Chmod(folder, 0o755); err != nil {
		t.Fatal(err)
	}
	log := &output{}
	code := run(context.Background(), []string{"device", "-state", stateDir(t), "-daemon-socket", filepath.Join(folder, "link.sock")}, stdio{strings.NewReader(""), io.Discard, log})
	if code == 0 || !strings.Contains(log.String(), "other users can reach") {
		t.Fatalf("exit %d, log %q; want a refusal that names the exposed path", code, log)
	}
}
