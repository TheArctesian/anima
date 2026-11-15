package hublink_test

import (
	"bytes"
	"context"
	"crypto/rand"
	"errors"
	"fmt"
	"io"
	"net"
	"net/http"
	"net/http/httptest"
	"path/filepath"
	"strconv"
	"testing"
	"time"

	"github.com/coder/websocket"

	"github.com/TheArctesian/anima/link/devices"
	"github.com/TheArctesian/anima/link/hublink"
	"github.com/TheArctesian/anima/link/identity"
	"github.com/TheArctesian/anima/link/statedir"
	"github.com/TheArctesian/anima/link/tunnel"
	"github.com/TheArctesian/anima/link/tunnel/testbed"
)

func TestMain(m *testing.M) { testbed.Main(m) }

// sessionsEndedOnTheHub gets one value each time the fake hub sees a
// WebSocket end: the connection from the hub link to the hub is closed.
var sessionsEndedOnTheHub = make(chan struct{}, 64)

// fakeHub stands in for the hub: plain HTTP and a WebSocket echo on loopback.
func fakeHub(t *testing.T) string {
	t.Helper()
	mux := http.NewServeMux()
	mux.HandleFunc("POST /count", func(w http.ResponseWriter, r *http.Request) {
		n, _ := io.Copy(io.Discard, r.Body)
		fmt.Fprint(w, n)
	})
	mux.HandleFunc("GET /blob", func(w http.ResponseWriter, r *http.Request) {
		n, _ := strconv.Atoi(r.URL.Query().Get("bytes"))
		w.Write(bytes.Repeat([]byte{0xA5}, n))
	})
	mux.HandleFunc("GET /ws", func(w http.ResponseWriter, r *http.Request) {
		c, err := websocket.Accept(w, r, nil)
		if err != nil {
			return
		}
		defer c.CloseNow()
		defer func() { sessionsEndedOnTheHub <- struct{}{} }()
		for {
			kind, message, err := c.Read(r.Context())
			if err != nil || c.Write(r.Context(), kind, message) != nil {
				return
			}
		}
	})
	server := httptest.NewServer(mux)
	t.Cleanup(server.Close)
	return server.Listener.Addr().String()
}

// rig is a running hub link with its list of devices, in front of a fake hub.
type rig struct {
	link  *hublink.Link
	store *devices.Store
}

func start(t *testing.T) rig {
	t.Helper()
	return startInFrontOf(t, fakeHub(t), false)
}

// startInFrontOf starts a hub link that sends streams to the given target.
func startInFrontOf(t *testing.T, target string, nameDevice bool) rig {
	t.Helper()
	dir, err := statedir.Open(filepath.Join(t.TempDir(), "s"))
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { dir.Close() })
	store, err := devices.Open(dir, time.Now)
	if err != nil {
		t.Fatal(err)
	}
	ctx, cancel := context.WithTimeout(context.Background(), 20*time.Second)
	defer cancel()
	link, err := hublink.Start(ctx, hublink.Config{
		Identity:   identity.Hub{Key: tunnel.NewPrivateKey(), PresharedKey: tunnel.NewPresharedKey(), Relay: testbed.StartRelay(t)},
		Target:     target,
		NameDevice: nameDevice,
		Devices:    store,
		Sweep:      200 * time.Millisecond,
	})
	if err != nil {
		t.Fatalf("Start: %v", err)
	}
	t.Cleanup(func() { link.Close() })
	return rig{link, store}
}

// device makes the device side for a key and an HTTP client that reaches the
// hub through it.
func (r rig) device(t *testing.T, key tunnel.PrivateKey) (*tunnel.Device, *http.Client) {
	t.Helper()
	device, err := tunnel.NewDevice(tunnel.DeviceConfig{Key: key, Hub: r.link.Address()})
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { device.Close() })
	transport := &http.Transport{DialContext: func(ctx context.Context, _, _ string) (net.Conn, error) {
		return device.Dial(ctx, tunnel.HubPort)
	}}
	t.Cleanup(transport.CloseIdleConnections)
	return device, &http.Client{Transport: transport, Timeout: 30 * time.Second}
}

func (r rig) permit(t *testing.T, key tunnel.PrivateKey) {
	t.Helper()
	if _, err := r.store.Permit(devices.Device{Key: key.Public(), Name: "test device"}); err != nil {
		t.Fatal(err)
	}
}

func TestLNK002AKeyOutsideTheListGetsNoAnswerAndWorksOnceItIsPermitted(t *testing.T) {
	r := start(t)
	key := tunnel.NewPrivateKey()
	device, client := r.device(t, key)

	ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
	err := device.Register(ctx)
	cancel()
	if !errors.Is(err, context.DeadlineExceeded) {
		t.Fatalf("a key outside the list: Register = %v, want no answer", err)
	}

	r.permit(t, key)
	response, err := client.Get("http://hub/blob?bytes=10")
	if err != nil {
		t.Fatalf("after the key was permitted, with no restart: %v", err)
	}
	response.Body.Close()
}

func TestHTTPPassesInBothDirectionsWithEveryByte(t *testing.T) {
	r := start(t)
	key := tunnel.NewPrivateKey()
	r.permit(t, key)
	_, client := r.device(t, key)
	const size = 1 << 20

	body := make([]byte, size)
	rand.Read(body)
	response, err := client.Post("http://hub/count", "application/octet-stream", bytes.NewReader(body))
	if err != nil {
		t.Fatal(err)
	}
	counted, _ := io.ReadAll(response.Body)
	response.Body.Close()
	if string(counted) != strconv.Itoa(size) {
		t.Errorf("the hub got %s bytes of an upload of %d", counted, size)
	}

	response, err = client.Get("http://hub/blob?bytes=" + strconv.Itoa(size))
	if err != nil {
		t.Fatal(err)
	}
	got, err := io.ReadAll(response.Body)
	response.Body.Close()
	if err != nil || len(got) != size || !bytes.Equal(got, bytes.Repeat([]byte{0xA5}, size)) {
		t.Errorf("the device got %d bytes of a download of %d (%v)", len(got), size, err)
	}
}

func TestAWebSocketPassesAndCountsAsAStreamOfItsDevice(t *testing.T) {
	r := start(t)
	key, other := tunnel.NewPrivateKey(), tunnel.NewPrivateKey()
	r.permit(t, key)
	_, client := r.device(t, key)
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()

	socket, _, err := websocket.Dial(ctx, "ws://hub/ws", &websocket.DialOptions{HTTPClient: client})
	if err != nil {
		t.Fatal(err)
	}
	for i := range 50 {
		message := []byte(fmt.Sprintf("message %d", i))
		if err := socket.Write(ctx, websocket.MessageBinary, message); err != nil {
			t.Fatal(err)
		}
		if _, got, err := socket.Read(ctx); err != nil || !bytes.Equal(got, message) {
			t.Fatalf("echo %d = %q, %v", i, got, err)
		}
	}
	if got := r.link.Streams(key.Public()); got != 1 {
		t.Errorf("open streams of the device = %d, want 1", got)
	}
	if got := r.link.Streams(other.Public()); got != 0 {
		t.Errorf("open streams of another device = %d, want 0", got)
	}

	socket.Close(websocket.StatusNormalClosure, "")
	deadline := time.Now().Add(5 * time.Second)
	for r.link.Streams(key.Public()) != 0 {
		if time.Now().After(deadline) {
			t.Fatal("the stream is still recorded 5 s after the WebSocket closed")
		}
		time.Sleep(20 * time.Millisecond)
	}
}

func TestTheTargetMustBeOnLoopback(t *testing.T) {
	for _, target := range []string{"192.0.2.10:8443", "hub.example:8443", "127.0.0.1", "", "0.0.0.0:8443", "127.0.0.1:0", "[2001:db8::1]:8443", "hub/link.sock", "unix:/run/hub/link.sock"} {
		_, err := hublink.Start(context.Background(), hublink.Config{
			Identity: identity.Hub{Key: tunnel.NewPrivateKey(), PresharedKey: tunnel.NewPresharedKey()},
			Target:   target,
		})
		if !errors.Is(err, hublink.ErrTarget) {
			t.Errorf("target %q: err = %v, want ErrTarget", target, err)
		}
	}
}

func TestCloseEndsTheOpenStreamsOnBothSides(t *testing.T) {
	r := start(t)
	key := tunnel.NewPrivateKey()
	r.permit(t, key)
	_, client := r.device(t, key)
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()
	socket, _, err := websocket.Dial(ctx, "ws://hub/ws", &websocket.DialOptions{HTTPClient: client})
	if err != nil {
		t.Fatal(err)
	}
	defer socket.CloseNow()

	closed := make(chan struct{})
	go func() { r.link.Close(); close(closed) }()
	select {
	case <-closed:
	case <-time.After(10 * time.Second):
		t.Fatal("Close still waits 10 s after the call")
	}
	if got := r.link.Streams(key.Public()); got != 0 {
		t.Errorf("open streams after Close = %d, want 0", got)
	}
	if _, _, err := socket.Read(ctx); err == nil {
		t.Error("the device's WebSocket is still open after the hub link closed")
	}
}

func TestOneDeviceCannotTakeEveryStream(t *testing.T) {
	r := start(t)
	greedy, other := tunnel.NewPrivateKey(), tunnel.NewPrivateKey()
	r.permit(t, greedy)
	r.permit(t, other)
	device, _ := r.device(t, greedy)
	ctx, cancel := context.WithTimeout(context.Background(), 60*time.Second)
	defer cancel()

	// One stream more than the limit. The hub link closes the stream that is
	// over the limit; the others stay open and idle.
	for range hublink.MaxStreamsOfDevice + 1 {
		stream, err := device.Dial(ctx, tunnel.HubPort)
		if err != nil {
			t.Fatal(err)
		}
		defer stream.Close()
	}
	deadline := time.Now().Add(10 * time.Second)
	for r.link.Streams(greedy.Public()) != hublink.MaxStreamsOfDevice {
		if time.Now().After(deadline) {
			t.Fatalf("open streams of the device = %d, want the limit of %d", r.link.Streams(greedy.Public()), hublink.MaxStreamsOfDevice)
		}
		time.Sleep(50 * time.Millisecond)
	}

	_, client := r.device(t, other)
	response, err := client.Get("http://hub/blob?bytes=10")
	if err != nil {
		t.Fatalf("another device while one device is at its limit: %v", err)
	}
	response.Body.Close()
}
