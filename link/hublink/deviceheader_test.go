package hublink_test

import (
	"context"
	"io"
	"net"
	"net/http"
	"os"
	"path/filepath"
	"testing"
	"time"

	"github.com/TheArctesian/anima/link/hublink/testhub"
	"github.com/TheArctesian/anima/link/tunnel"
)

// rawHub is a loopback listener that gives each accepted connection to the test.
func rawHub(t *testing.T) (target string, accepted chan net.Conn) {
	t.Helper()
	listener, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { listener.Close() })
	accepted = make(chan net.Conn, 4)
	go func() {
		for {
			conn, err := listener.Accept()
			if err != nil {
				return
			}
			accepted <- conn
		}
	}()
	return listener.Addr().String(), accepted
}

// sendHello opens a stream from a permitted device and writes "hello" on
// it. It returns what the hub side of the link got.
func sendHello(t *testing.T, r rig, accepted chan net.Conn, key tunnel.PrivateKey) net.Conn {
	t.Helper()
	device, _ := r.device(t, key)
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()
	stream, err := device.Dial(ctx, tunnel.HubPort)
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { stream.Close() })
	if _, err := stream.Write([]byte("hello")); err != nil {
		t.Fatal(err)
	}
	select {
	case atHub := <-accepted:
		t.Cleanup(func() { atHub.Close() })
		atHub.SetReadDeadline(time.Now().Add(10 * time.Second))
		return atHub
	case <-time.After(10 * time.Second):
		t.Fatal("the hub got no connection")
		return nil
	}
}

func read(t *testing.T, from io.Reader, n int) []byte {
	t.Helper()
	buffer := make([]byte, n)
	if _, err := io.ReadFull(from, buffer); err != nil {
		t.Fatalf("read %d bytes: %v", n, err)
	}
	return buffer
}

func get(client *http.Client, path string) (string, error) {
	response, err := client.Get("http://hub" + path)
	if err != nil {
		return "", err
	}
	defer response.Body.Close()
	body, err := io.ReadAll(response.Body)
	return string(body), err
}

func TestLNK002TheHubsSocketGetsEachDeviceByItsOwnAddressAndNoOtherKey(t *testing.T) {
	hub := testhub.Start(t)
	r := startInFrontOf(t, hub.Socket)
	laptop, desk, stranger := tunnel.NewPrivateKey(), tunnel.NewPrivateKey(), tunnel.NewPrivateKey()
	r.permit(t, laptop)
	r.permit(t, desk)
	for _, key := range []tunnel.PrivateKey{laptop, desk} {
		_, client := r.device(t, key)
		want, _ := r.store.AddressOf(key.Public())
		if got, err := get(client, "/"); err != nil || got != want.String() {
			t.Errorf("the hub saw the device as %q (%v), want its link address %v", got, err, want)
		}
	}
	named := hub.Named()
	_, client := r.device(t, stranger)
	client.Timeout = 3 * time.Second
	if got, err := get(client, "/"); err == nil {
		t.Errorf("a key that is not in the list reached the hub: %q", got)
	}
	if hub.Named() != named {
		t.Error("the hub's socket got a connection for a key that is not in the list")
	}
}

func TestTheLinkDoesNotConnectToASocketThatOtherUsersCanReach(t *testing.T) {
	for name, expose := range map[string]func(*testhub.Hub) error{
		"the socket":        func(hub *testhub.Hub) error { return os.Chmod(hub.Socket, 0o666) },
		"the socket folder": func(hub *testhub.Hub) error { return os.Chmod(filepath.Dir(hub.Socket), 0o755) },
	} {
		t.Run(name, func(t *testing.T) {
			hub := testhub.Start(t)
			if err := expose(hub); err != nil {
				t.Fatal(err)
			}
			r := startInFrontOf(t, hub.Socket)
			key := tunnel.NewPrivateKey()
			r.permit(t, key)
			_, client := r.device(t, key)
			client.Timeout = 5 * time.Second
			if got, err := get(client, "/"); err == nil {
				t.Errorf("the link sent a device's request to a socket that other users can reach: %q", got)
			}
			if hub.Named() != 0 {
				t.Error("the exposed socket got a connection")
			}
		})
	}
}

func TestALoopbackTargetGetsOnlyTheDevicesBytes(t *testing.T) {
	target, accepted := rawHub(t)
	r := startInFrontOf(t, target)
	key := tunnel.NewPrivateKey()
	r.permit(t, key)
	atHub := sendHello(t, r, accepted, key)
	if got := read(t, atHub, 5); string(got) != "hello" {
		t.Errorf("the hub got %q first, want the device's bytes with nothing before them", got)
	}
}
