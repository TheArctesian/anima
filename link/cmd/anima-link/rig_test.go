package main

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"strings"
	"sync"
	"testing"
	"time"

	"github.com/TheArctesian/anima/link/identity"
	"github.com/TheArctesian/anima/link/statedir"
	"github.com/TheArctesian/anima/link/tunnel"
	"github.com/TheArctesian/anima/link/tunnel/testbed"
)

func TestMain(m *testing.M) { testbed.Main(m) }

const greeting = "the hub answers"

// seedRelay gives a new state directory a relay on loopback, as if an
// earlier start had chosen it. Without it the hub link would ask the
// internet for a relay map.
func seedRelay(t *testing.T, state string) {
	t.Helper()
	dir, err := statedir.Open(state)
	if err != nil {
		t.Fatal(err)
	}
	defer dir.Close()
	hub, err := identity.LoadHub(dir)
	if err != nil {
		t.Fatal(err)
	}
	hub.Relay = testbed.StartRelay(t)
	if err := hub.Save(dir); err != nil {
		t.Fatal(err)
	}
}

// fakeHub is the loopback server that the hub link sends streams to.
func fakeHub(t *testing.T) string {
	t.Helper()
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {
		fmt.Fprint(w, greeting)
	}))
	t.Cleanup(server.Close)
	return server.Listener.Addr().String()
}

// output is a buffer that the command and the test can use at the same time.
type output struct {
	mu   sync.Mutex
	text bytes.Buffer
}

func (o *output) Write(p []byte) (int, error) {
	o.mu.Lock()
	defer o.mu.Unlock()
	return o.text.Write(p)
}

func (o *output) String() string {
	o.mu.Lock()
	defer o.mu.Unlock()
	return o.text.String()
}

// stateDir is a short path: a Unix socket path has a limit of about 100 bytes,
// and the test's own temporary directory is too long on macOS.
func stateDir(t *testing.T) string {
	t.Helper()
	path, err := os.MkdirTemp("/tmp", "anima-link-")
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { os.RemoveAll(path) })
	return filepath.Join(path, "s")
}

// runningHub is a hub link that a test started.
type runningHub struct {
	state  string
	stdin  io.Closer
	log    *output
	exited chan int
}

func startHub(t *testing.T) runningHub {
	t.Helper()
	state := stateDir(t)
	seedRelay(t, state)
	return startHubOn(t, state)
}

// startHubOn starts a hub link on a state directory that has its relay.
func startHubOn(t *testing.T, state string) runningHub {
	t.Helper()
	return startHubInFrontOf(t, state, fakeHub(t))
}

// startHubInFrontOf starts a hub link that sends streams to a target.
func startHubInFrontOf(t *testing.T, state, target string) runningHub {
	t.Helper()
	stdinReader, stdinWriter := io.Pipe()
	hub := runningHub{state: state, stdin: stdinWriter, log: &output{}, exited: make(chan int, 1)}
	go func() {
		hub.exited <- run(context.Background(), []string{"hub", "-state", state, "-target", target, "-stop-with-stdin"}, stdio{stdinReader, io.Discard, hub.log})
	}()
	t.Cleanup(func() { stdinWriter.Close() })
	deadline := time.Now().Add(20 * time.Second)
	for !strings.Contains(hub.log.String(), "ready") {
		if time.Now().After(deadline) {
			t.Fatalf("the hub link did not start: %s", hub.log)
		}
		time.Sleep(10 * time.Millisecond)
	}
	return hub
}

// ctl runs one control verb as a second process would.
func (h runningHub) ctl(t *testing.T, args ...string) (stdout string, code int) {
	t.Helper()
	var out, errs output
	code = run(context.Background(), append([]string{"ctl", "-state", h.state}, args...), stdio{strings.NewReader(""), &out, &errs})
	return out.String() + errs.String(), code
}

// secretsOf reads every secret that a state directory holds, as the texts
// that would show in a log.
func secretsOf(t *testing.T, state string) []string {
	t.Helper()
	text, err := os.ReadFile(filepath.Join(state, "hub.json"))
	if err != nil {
		t.Fatal(err)
	}
	var saved struct{ Key, PresharedKey string }
	if err := json.Unmarshal(text, &saved); err != nil {
		t.Fatal(err)
	}
	if saved.Key == "" || saved.PresharedKey == "" {
		t.Fatal("the state file holds no key: the log test would prove nothing")
	}
	return []string{saved.Key, strings.TrimPrefix(saved.Key, "privkey:"), saved.PresharedKey}
}

// get asks the fake hub for its greeting through the link, as a device with
// the given key and the given hub address.
func get(t *testing.T, key tunnel.PrivateKey, address string, limit time.Duration) (string, error) {
	t.Helper()
	hub, err := tunnel.ParseAddress(address)
	if err != nil {
		t.Fatalf("the address from the control socket is not valid: %v", err)
	}
	device, err := tunnel.NewDevice(tunnel.DeviceConfig{Key: key, Hub: hub})
	if err != nil {
		t.Fatal(err)
	}
	defer device.Close()
	transport := &http.Transport{DialContext: func(ctx context.Context, _, _ string) (net.Conn, error) {
		return device.Dial(ctx, tunnel.HubPort)
	}}
	defer transport.CloseIdleConnections()
	response, err := (&http.Client{Transport: transport, Timeout: limit}).Get("http://hub/")
	if err != nil {
		return "", err
	}
	defer response.Body.Close()
	body, err := io.ReadAll(response.Body)
	return string(body), err
}

// address asks the hub link for its address, as the operator does.
func (h runningHub) address(t *testing.T) string {
	t.Helper()
	text, code := h.ctl(t, "address.show")
	var address string
	if err := json.Unmarshal([]byte(text), &address); code != 0 || err != nil {
		t.Fatalf("ctl address.show: exit %d", code)
	}
	return address
}

func (h runningHub) permit(t *testing.T, key tunnel.PrivateKey, name string) (string, int) {
	t.Helper()
	keyText, _ := key.Public().MarshalText()
	return h.ctl(t, "devices.permit", fmt.Sprintf(`{"key":%q,"name":%q}`, keyText, name))
}

// startDevice starts a device link, as a program, for a device that already
// has its key permitted on the hub and the hub's address saved.
// startDevice starts a device link that the hub permits; more is more
// arguments of the command "device".
func startDevice(t *testing.T, hub runningHub, more ...string) (state, local string) {
	t.Helper()
	state = stateDir(t)
	dir, err := statedir.Open(state)
	if err != nil {
		t.Fatal(err)
	}
	device, err := identity.LoadDevice(dir)
	if err != nil {
		t.Fatal(err)
	}
	if device.Hub, err = tunnel.ParseAddress(hub.address(t)); err != nil {
		t.Fatal(err)
	}
	if err := device.Save(dir); err != nil {
		t.Fatal(err)
	}
	dir.Close()
	if text, code := hub.permit(t, device.Key, "device link"); code != 0 {
		t.Fatalf("devices.permit: %q, exit %d", text, code)
	}

	stdinReader, stdinWriter := io.Pipe()
	log := &output{}
	go run(context.Background(), append([]string{"device", "-state", state, "-stop-with-stdin"}, more...), stdio{stdinReader, io.Discard, log})
	t.Cleanup(func() { stdinWriter.Close() })
	deadline := time.Now().Add(20 * time.Second)
	for !strings.Contains(log.String(), "ready") {
		if time.Now().After(deadline) {
			t.Fatalf("the device link did not start: %s", log)
		}
		time.Sleep(10 * time.Millisecond)
	}
	text, code := runningHub{state: state}.ctl(t, "status")
	var status struct{ Role, Local string }
	if err := json.Unmarshal([]byte(text), &status); code != 0 || err != nil || status.Role != "device" {
		t.Fatalf("device status = %q, exit %d", text, code)
	}
	return state, status.Local
}

func loadDevice(t *testing.T, state string) identity.Device {
	t.Helper()
	dir, err := statedir.Open(state)
	if err != nil {
		t.Fatal(err)
	}
	defer dir.Close()
	device, err := identity.LoadDevice(dir)
	if err != nil {
		t.Fatal(err)
	}
	return device
}
