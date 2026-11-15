package hublink_test

import (
	"bytes"
	"context"
	"encoding/binary"
	"io"
	"net"
	"testing"
	"time"

	"github.com/TheArctesian/anima/link/hublink"
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

func TestTheHubIsToldTheDeviceKeyOfEachStreamWhenItAsksForIt(t *testing.T) {
	target, accepted := rawHub(t)
	r := startInFrontOf(t, target, true)
	for range 2 { // two devices: each stream names its own
		key := tunnel.NewPrivateKey()
		r.permit(t, key)
		atHub := sendHello(t, r, accepted, key)

		start := read(t, atHub, 16)
		signature := []byte{0x0D, 0x0A, 0x0D, 0x0A, 0x00, 0x0D, 0x0A, 0x51, 0x55, 0x49, 0x54, 0x0A}
		if !bytes.Equal(start[:12], signature) || start[12] != 0x21 || start[13] != 0x21 {
			t.Fatalf("the stream does not start with a PROXY version 2 header for TCP over IPv6: % x", start)
		}
		rest := read(t, atHub, int(binary.BigEndian.Uint16(start[14:16])))
		field := rest[36:] // after two IPv6 addresses and two ports
		if field[0] != hublink.DeviceKeyField || int(binary.BigEndian.Uint16(field[1:3])) != len(field)-3 {
			t.Fatalf("the header's extra field is not the device key field: % x", field[:3])
		}
		var named tunnel.DeviceKey
		if err := named.UnmarshalText(field[3:]); err != nil || named != key.Public() {
			t.Errorf("the header names the device %s, want %s", named, key.Public())
		}
		if got := read(t, atHub, 5); string(got) != "hello" {
			t.Errorf("after the header the hub got %q, want the device's bytes", got)
		}
	}
}

func TestTheHubGetsOnlyTheDevicesBytesWhenItDoesNotAskForTheKey(t *testing.T) {
	target, accepted := rawHub(t)
	r := startInFrontOf(t, target, false)
	key := tunnel.NewPrivateKey()
	r.permit(t, key)
	atHub := sendHello(t, r, accepted, key)
	if got := read(t, atHub, 5); string(got) != "hello" {
		t.Errorf("the hub got %q first, want the device's bytes with nothing before them", got)
	}
}
