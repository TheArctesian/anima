package hublink_test

import (
	"context"
	"errors"
	"io"
	"net"
	"testing"
	"time"

	"github.com/coder/websocket"

	"github.com/TheArctesian/anima/link/devices"
	"github.com/TheArctesian/anima/link/presence"
	"github.com/TheArctesian/anima/link/tunnel"
)

// session opens a WebSocket through the link and checks that it works.
func (r rig) session(t *testing.T, key tunnel.PrivateKey) *websocket.Conn {
	t.Helper()
	_, client := r.device(t, key)
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()
	socket, _, err := websocket.Dial(ctx, "ws://hub/ws", &websocket.DialOptions{HTTPClient: client})
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { socket.CloseNow() })
	echoOnce(t, socket)
	return socket
}

func echoOnce(t *testing.T, socket *websocket.Conn) {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()
	if err := socket.Write(ctx, websocket.MessageText, []byte("still here")); err != nil {
		t.Fatalf("write: %v", err)
	}
	if _, _, err := socket.Read(ctx); err != nil {
		t.Fatalf("read: %v", err)
	}
}

// ended reports when a session ends for the device.
func ended(session *websocket.Conn) <-chan time.Time {
	at := make(chan time.Time, 1)
	go func() {
		session.Read(context.Background())
		at <- time.Now()
	}()
	return at
}

func drain(events chan struct{}) {
	for len(events) > 0 {
		<-events
	}
}

func TestLNK004RevocationEndsTheOpenSessionOfThatDeviceOnly(t *testing.T) {
	r := start(t)
	revoked, kept := tunnel.NewPrivateKey(), tunnel.NewPrivateKey()
	r.permit(t, revoked)
	r.permit(t, kept)
	revokedSession := r.session(t, revoked)
	keptSession := r.session(t, kept)
	drain(sessionsEndedOnTheHub)

	sessionEnded := ended(revokedSession)
	commanded := time.Now()
	closed, err := r.link.Revoke(revoked.Public())
	if err != nil || closed != 1 {
		t.Fatalf("Revoke = %d, %v; want 1 stream closed", closed, err)
	}

	select {
	case at := <-sessionEnded:
		if took := at.Sub(commanded); took > time.Second {
			t.Errorf("the device's session ended %v after the revocation, want under 1 s", took)
		}
	case <-time.After(5 * time.Second):
		t.Fatal("the device's session is still open 5 s after the revocation")
	}
	select {
	case <-sessionsEndedOnTheHub:
	case <-time.After(5 * time.Second):
		t.Error("the hub link's connection to the hub is still open 5 s after the revocation")
	}
	for deadline := time.Now().Add(5 * time.Second); r.link.Streams(revoked.Public()) != 0; time.Sleep(20 * time.Millisecond) {
		if time.Now().After(deadline) {
			t.Fatalf("the hub link still has %d streams of the revoked device 5 s after the revocation", r.link.Streams(revoked.Public()))
		}
	}
	if r.store.Permits(revoked.Public()) {
		t.Error("the revoked key is still in the list")
	}

	// The tunnel forgets the device half a second after its streams were
	// closed. Until then the device still gets the tunnel's own answer, but
	// no stream (see the next test). After it, the device gets no answer.
	time.Sleep(time.Second)
	device, _ := r.device(t, revoked)
	ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
	defer cancel()
	if err := device.Register(ctx); !errors.Is(err, context.DeadlineExceeded) {
		t.Errorf("the revoked device connects again: Register = %v, want no answer", err)
	}

	echoOnce(t, keptSession)
	if _, err := r.link.Revoke(revoked.Public()); !errors.Is(err, devices.ErrUnknown) {
		t.Errorf("a second Revoke = %v, want ErrUnknown", err)
	}
}

// A revocation can be cut after the key left the list and before the device
// was disconnected, for example when the hub link is killed. The device's
// tunnel connection is then still up.
func TestLNK004ADeviceThatLeftTheListGetsNoNewStreamAndIsSweptOut(t *testing.T) {
	r := start(t)
	key := tunnel.NewPrivateKey()
	r.permit(t, key)
	_, client := r.device(t, key)
	if response, err := client.Get("http://hub/blob?bytes=10"); err != nil {
		t.Fatal(err)
	} else {
		response.Body.Close()
	}
	session := r.session(t, key)
	sessionEnded := ended(session)

	if _, err := r.store.Revoke(key.Public()); err != nil {
		t.Fatal(err)
	}

	client.CloseIdleConnections()
	client.Timeout = 3 * time.Second
	if response, err := client.Get("http://hub/blob?bytes=10"); err == nil {
		response.Body.Close()
		t.Error("a device that left the list opened a new stream to the hub")
	}
	select {
	case <-sessionEnded:
	case <-time.After(10 * time.Second):
		t.Fatal("the open session of a device that left the list was not swept out in 10 s (sweep: 200 ms)")
	}
}

// reverse opens a stream to the reverse port as a device does, writes one
// line "here" and waits until the hub link has recorded it.
func (r rig) reverse(t *testing.T, device *tunnel.Device, key tunnel.PrivateKey) net.Conn {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()
	stream, err := device.Dial(ctx, tunnel.ReversePort)
	if err != nil {
		t.Fatalf("a stream to the reverse port: %v", err)
	}
	t.Cleanup(func() { stream.Close() })
	address, _ := r.store.AddressOf(key.Public())
	before, _ := r.link.Seen(address)
	io.WriteString(stream, presence.Here)
	deadline := time.Now().Add(10 * time.Second)
	for {
		if at, seen := r.link.Seen(address); seen && at.After(before) {
			return stream
		}
		if time.Now().After(deadline) {
			t.Fatal("the hub link did not record the line of the device in 10 s")
		}
		time.Sleep(20 * time.Millisecond)
	}
}

// endedByTheHub reports whether the hub link ends a stream within 10 s.
func endedByTheHub(stream net.Conn) bool {
	_ = stream.SetReadDeadline(time.Now().Add(10 * time.Second))
	_, err := stream.Read(make([]byte, 1))
	var timeout net.Error
	return err != nil && !(errors.As(err, &timeout) && timeout.Timeout())
}

func TestHMD047TheHubLinkSeesAPermittedDeviceOnOneStreamOfTheReversePort(t *testing.T) {
	r := start(t)
	key, other := tunnel.NewPrivateKey(), tunnel.NewPrivateKey()
	r.permit(t, key)
	r.permit(t, other)
	device, _ := r.device(t, key)
	address, _ := r.store.AddressOf(key.Public())
	if at, seen := r.link.Seen(address); seen {
		t.Fatalf("a device that wrote no line was seen at %v", at)
	}

	first := r.reverse(t, device, key)
	second := r.reverse(t, device, key)

	if !endedByTheHub(first) {
		t.Error("the first stream is still open 10 s after the second one took its place")
	}
	if got := r.link.Streams(key.Public()); got != 1 {
		t.Errorf("open streams of the device = %d, want its one stream to the reverse port", got)
	}
	silent, _ := r.store.AddressOf(other.Public())
	if at, seen := r.link.Seen(silent); seen {
		t.Errorf("another device, which wrote no line, was seen at %v", at)
	}
	io.WriteString(second, "open 22\n")
	if !endedByTheHub(second) {
		t.Error("the stream is still open 10 s after a line that is not \"here\"")
	}
}

func TestHMD047ARevocationClosesTheStreamToTheReversePort(t *testing.T) {
	r := start(t)
	key := tunnel.NewPrivateKey()
	r.permit(t, key)
	device, _ := r.device(t, key)
	stream := r.reverse(t, device, key)

	closed, err := r.link.Revoke(key.Public())
	if err != nil || closed != 1 {
		t.Fatalf("Revoke = %d, %v; want the one stream closed", closed, err)
	}
	if !endedByTheHub(stream) {
		t.Error("the stream is still open 10 s after the revocation")
	}
}
