package hublink_test

import (
	"context"
	"errors"
	"testing"
	"time"

	"github.com/coder/websocket"

	"github.com/TheArctesian/anima/link/devices"
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
	if got := r.link.Streams(revoked.Public()); got != 0 {
		t.Errorf("the hub link still has %d streams of the revoked device", got)
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
