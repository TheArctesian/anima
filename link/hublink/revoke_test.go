package hublink_test

import (
	"context"
	"errors"
	"testing"
	"time"

	"github.com/coder/websocket"

	"github.com/TheArctesian/anima/link/hublink"
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

func TestLNK004RevocationEndsTheOpenSessionOfThatDeviceOnly(t *testing.T) {
	r := start(t)
	revoked, kept := tunnel.NewPrivateKey(), tunnel.NewPrivateKey()
	r.permit(t, revoked)
	r.permit(t, kept)
	revokedSession := r.session(t, revoked)
	keptSession := r.session(t, kept)

	ended := make(chan time.Time, 1)
	go func() {
		revokedSession.Read(context.Background())
		ended <- time.Now()
	}()
	commanded := time.Now()
	if err := r.store.Revoke(revoked.Public()); err != nil {
		t.Fatal(err)
	}
	closed, err := r.link.Disconnect(revoked.Public())
	if err != nil || closed != 1 {
		t.Fatalf("Disconnect = %d, %v; want 1 stream closed", closed, err)
	}

	select {
	case at := <-ended:
		if took := at.Sub(commanded); took > time.Second {
			t.Errorf("the device's session ended %v after the revocation, want under 1 s", took)
		}
	case <-time.After(5 * time.Second):
		t.Fatal("the device's session is still open 5 s after the revocation")
	}
	if got := r.link.Streams(revoked.Public()); got != 0 {
		t.Errorf("the hub link still has %d streams of the revoked device", got)
	}

	device, _ := r.device(t, revoked)
	ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
	defer cancel()
	if err := device.Register(ctx); !errors.Is(err, context.DeadlineExceeded) {
		t.Errorf("the revoked device connects again: Register = %v, want no answer", err)
	}

	echoOnce(t, keptSession)
}

func TestLNK004ADeviceInTheListIsNotDisconnected(t *testing.T) {
	r := start(t)
	key := tunnel.NewPrivateKey()
	r.permit(t, key)
	session := r.session(t, key)

	if _, err := r.link.Disconnect(key.Public()); !errors.Is(err, hublink.ErrStillPermitted) {
		t.Fatalf("Disconnect of a permitted device: err = %v, want ErrStillPermitted", err)
	}
	echoOnce(t, session)
}
