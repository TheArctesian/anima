package devicelink_test

import (
	"context"
	"errors"
	"io"
	"net"
	"testing"
	"time"

	"github.com/TheArctesian/anima/link/dial"
	"github.com/TheArctesian/anima/link/hublink"
	"github.com/TheArctesian/anima/link/tunnel"
)

// seenBy waits until a hub link has seen the site's device, and returns how
// long it took.
func (s *site) seenBy(t *testing.T, hub *hublink.Link, limit time.Duration) time.Duration {
	t.Helper()
	address, permitted := s.permitted.AddressOf(s.device.Key.Public())
	if !permitted {
		t.Fatal("the device is not in the list")
	}
	start := time.Now()
	for {
		if _, seen := hub.Seen(address); seen {
			return time.Since(start)
		}
		if time.Since(start) > limit {
			t.Fatalf("the hub link did not see the device in %v", limit)
		}
		time.Sleep(100 * time.Millisecond)
	}
}

func TestHMD047ARunningDeviceLinkIsSeenByItsHubAndAgainAfterAHubRestart(t *testing.T) {
	s := newSite(t)
	hub := s.startHub(t)
	device := s.startDevice(t, time.Second)
	s.seenBy(t, hub, 30*time.Second)
	address, _ := s.permitted.AddressOf(s.device.Key.Public())
	first, _ := hub.Seen(address)
	deadline := time.Now().Add(10 * time.Second)
	for {
		if again, _ := hub.Seen(address); again.After(first) {
			break
		}
		if time.Now().After(deadline) {
			t.Fatal("the device wrote no second line in 10 s, with a keepalive of 1 s")
		}
		time.Sleep(100 * time.Millisecond)
	}

	hub.Close()
	restarted := s.startHub(t)

	took := s.seenBy(t, restarted, 45*time.Second)
	t.Logf("the restarted hub link saw the device after %v", took.Round(time.Millisecond))
	getUntil(t, device, 30*time.Second)
}

// A hub link that is older than the reverse port serves the hub port only.
func TestHMD047AHubWithNoReversePortStillAnswersThroughTheLocalPort(t *testing.T) {
	s := newSite(t)
	ctx, cancel := context.WithTimeout(context.Background(), 20*time.Second)
	defer cancel()
	old, err := tunnel.StartHub(ctx, tunnel.HubConfig{
		Key: s.hub.Key, PresharedKey: s.hub.PresharedKey, Relay: s.hub.Relay,
		Admit: func(tunnel.DeviceKey) bool { return true },
	})
	if err != nil {
		t.Fatalf("StartHub: %v", err)
	}
	t.Cleanup(func() { old.Close() })
	listener, err := old.Listen(tunnel.HubPort)
	if err != nil {
		t.Fatal(err)
	}
	go func() {
		for {
			stream, err := listener.Accept()
			if err != nil {
				return
			}
			go func() {
				target, err := net.Dial("tcp", s.target)
				if err != nil {
					stream.Close()
					return
				}
				tunnel.Join(stream, target)
			}()
		}
	}()
	s.device.Hub = old.Address()
	device := s.startDevice(t, 500*time.Millisecond)

	getUntil(t, device, 30*time.Second)
	// The device has asked for the reverse port and was refused by now.
	time.Sleep(2 * time.Second)
	getUntil(t, device, 10*time.Second)

	closed := make(chan struct{})
	go func() { device.Close(); close(closed) }()
	select {
	case <-closed:
	case <-time.After(5 * time.Second):
		t.Fatal("Close still waits 5 s after the call, with a hub that has no reverse port")
	}
}

// localPort is a port on loopback of this computer. With answers, a program
// on it sends back what it reads; without, each connection waits unread.
func localPort(t *testing.T, answers bool) uint16 {
	t.Helper()
	listener, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { listener.Close() })
	if answers {
		go func() {
			for {
				conn, err := listener.Accept()
				if err != nil {
					return
				}
				go func() { _, _ = io.Copy(conn, conn); conn.Close() }()
			}
		}()
	}
	return uint16(listener.Addr().(*net.TCPAddr).Port)
}

// The whole path over a real tunnel: the hub link asks on the device's
// stream of the reverse port, and the device gives a stream to its local
// port, only for a port of its list.
func TestHMD048TheHubLinkReachesOnlyTheOpenLocalPortsOfADevice(t *testing.T) {
	open, shut := localPort(t, true), localPort(t, false)
	s := newSite(t)
	s.openPort = func(port uint16) bool { return port == open }
	hub := s.startHub(t)
	s.startDevice(t, time.Second)
	s.seenBy(t, hub, 30*time.Second)
	address, _ := s.permitted.AddressOf(s.device.Key.Public())
	ctx, cancel := context.WithTimeout(context.Background(), 20*time.Second)
	defer cancel()

	stream, release, err := hub.Open(ctx, address, open)
	if err != nil {
		t.Fatalf("Open of the open port: %v", err)
	}
	_ = stream.SetDeadline(time.Now().Add(10 * time.Second))
	if _, err := io.WriteString(stream, "from the hub"); err != nil {
		t.Fatal(err)
	}
	got := make([]byte, len("from the hub"))
	if _, err := io.ReadFull(stream, got); err != nil || string(got) != "from the hub" {
		t.Fatalf("the local port answered %q, %v", got, err)
	}
	stream.Close()
	release()

	if _, _, err := hub.Open(ctx, address, shut); !errors.Is(err, dial.ErrRefused) {
		t.Fatalf("Open of a port that is not in the list: %v, want ErrRefused", err)
	}
	if _, _, err := hub.Open(ctx, address.Next(), open); !errors.Is(err, dial.ErrNotConnected) {
		t.Fatalf("Open on a device that is not connected: %v, want ErrNotConnected", err)
	}
}
