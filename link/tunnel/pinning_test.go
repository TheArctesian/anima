package tunnel_test

// Pinning tests: each one states a behaviour of the Tailcat library that the
// link relies on (UPSTREAM.md). If an update of the library changes one, the
// test fails here and not in the field.

import (
	"context"
	"encoding/json"
	"errors"
	"net"
	"testing"
	"time"

	"github.com/TheArctesian/anima/link/tunnel"
)

func register(device *tunnel.Device, limit time.Duration) error {
	ctx, cancel := context.WithTimeout(context.Background(), limit)
	defer cancel()
	return device.Register(ctx)
}

func TestPinARefusedDeviceGetsNoAnswerUntilItIsAdmitted(t *testing.T) {
	var permitted admission
	hub := startHub(t, newIdentity(t), permitted.admit)
	serveEcho(t, hub)
	key := tunnel.NewPrivateKey()
	device := newDevice(t, hub, key)

	if err := register(device, 3*time.Second); !errors.Is(err, context.DeadlineExceeded) {
		t.Fatalf("refused device: Register = %v, want a timeout with no answer", err)
	}
	if _, known := hub.Paths()[key.Public()]; known {
		t.Error("the hub lists a device that it refused")
	}

	permitted.Store(true)
	if err := register(device, 10*time.Second); err != nil {
		t.Fatalf("after the key was admitted: Register = %v", err)
	}
	echo(t, dial(t, device), "admitted while the hub runs")
}

func TestPinTheHubKnowsTheKeyOfEachStream(t *testing.T) {
	hub := startHub(t, newIdentity(t), admitAll)
	ln, err := hub.Listen(echoPort)
	if err != nil {
		t.Fatal(err)
	}
	defer ln.Close()

	for range 2 {
		key := tunnel.NewPrivateKey()
		device := newDevice(t, hub, key)
		go func() {
			ctx, cancel := context.WithTimeout(context.Background(), 20*time.Second)
			defer cancel()
			if _, err := device.Dial(ctx, echoPort); err != nil {
				t.Errorf("Dial: %v", err)
			}
		}()
		stream, err := ln.Accept()
		if err != nil {
			t.Fatal(err)
		}
		got, ok := hub.DeviceOf(stream.RemoteAddr())
		stream.Close()
		if !ok || got != key.Public() {
			t.Errorf("the hub names device %s for the stream (known: %v), want %s", got.Fingerprint(), ok, key.Public().Fingerprint())
		}
	}
}

func TestPinDisconnectStopsTrafficButLeavesTheStreamOpen(t *testing.T) {
	var permitted admission
	permitted.Store(true)
	hub := startHub(t, newIdentity(t), permitted.admit)
	serveEcho(t, hub)
	key := tunnel.NewPrivateKey()
	stream := dial(t, newDevice(t, hub, key))
	echo(t, stream, "before")

	permitted.Store(false)
	if !hub.Disconnect(key.Public()) {
		t.Fatal("Disconnect = false for a connected device")
	}
	if hub.Disconnect(key.Public()) {
		t.Error("a second Disconnect = true")
	}

	if err := stream.SetDeadline(time.Now().Add(2 * time.Second)); err != nil {
		t.Fatal(err)
	}
	if _, err := stream.Write([]byte("after")); err != nil {
		t.Fatalf("write after Disconnect: %v", err)
	}
	_, err := stream.Read(make([]byte, 5))
	var timeout net.Error
	if !errors.As(err, &timeout) || !timeout.Timeout() {
		t.Errorf("read after Disconnect = %v, want a timeout: the library stops traffic and does not close the stream", err)
	}
}

func TestPinTheAddressIsTheSameAfterARestartFromSavedState(t *testing.T) {
	id := newIdentity(t)
	first := startHub(t, id, admitAll)
	address := first.Address()

	saved := struct {
		Key   tunnel.PrivateKey
		PSK   tunnel.PresharedKey
		Relay tunnel.Relay
	}{id.key, id.psk, first.Relay()}
	text, err := json.Marshal(saved)
	if err != nil {
		t.Fatal(err)
	}
	first.Close()
	saved.Key, saved.PSK, saved.Relay = tunnel.PrivateKey{}, tunnel.PresharedKey{}, tunnel.Relay{}
	if err := json.Unmarshal(text, &saved); err != nil {
		t.Fatal(err)
	}

	second := startHub(t, identity{saved.Key, saved.PSK, saved.Relay}, admitAll)
	if second.Address() != address {
		t.Error("the address changed after a restart with the saved key, pre-shared key and relay")
	}

	other := startHub(t, identity{saved.Key, tunnel.NewPresharedKey(), saved.Relay}, admitAll)
	if other.Address() == address {
		t.Error("a different pre-shared key gave the same address")
	}
}
