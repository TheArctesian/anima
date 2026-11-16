package devicelink_test

import (
	"bufio"
	"io"
	"net"
	"net/http"
	"net/netip"
	"testing"
	"time"

	"github.com/TheArctesian/anima/link/devicelink"
	"github.com/TheArctesian/anima/link/hublink/testhub"
	"github.com/TheArctesian/anima/link/identity"
	"github.com/TheArctesian/anima/link/tunnel"
)

// These tests put the hub link in front of a hub's socket, as plan item S5
// runs it, and check each rule of the link by what the hub sees: which
// device, by its link address, and no other.

// seenAs asks the hub, through a device's local port, which device it sees.
func seenAs(link *devicelink.Link, limit time.Duration) (netip.Addr, error) {
	text, err := get(link, limit)
	if err != nil {
		return netip.Addr{}, err
	}
	return netip.ParseAddr(text)
}

// seenUntil repeats the question until the hub answers.
func seenUntil(t *testing.T, link *devicelink.Link) netip.Addr {
	t.Helper()
	deadline := time.Now().Add(30 * time.Second)
	for {
		device, err := seenAs(link, 5*time.Second)
		if err == nil {
			return device
		}
		if time.Now().After(deadline) {
			t.Fatalf("the hub did not answer through the local port: %v", err)
		}
		time.Sleep(250 * time.Millisecond)
	}
}

func startJoined(t *testing.T, device identity.Device) *devicelink.Link {
	t.Helper()
	link, err := devicelink.Start(devicelink.Config{Identity: device, Listen: "127.0.0.1:0", Keepalive: 300 * time.Millisecond})
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { link.Close() })
	return link
}

// hold opens a request that the hub keeps open, as a session.
func hold(t *testing.T, link *devicelink.Link) net.Conn {
	t.Helper()
	session, err := net.DialTimeout("tcp", link.LocalAddr().String(), 5*time.Second)
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { session.Close() })
	session.SetDeadline(time.Now().Add(30 * time.Second))
	io.WriteString(session, "GET "+testhub.HoldPath+" HTTP/1.1\r\nHost: hub\r\n\r\n")
	response, err := http.ReadResponse(bufio.NewReader(session), nil)
	if err != nil {
		t.Fatalf("the hub did not open the session: %v", err)
	}
	if body, _ := io.ReadAll(response.Body); string(body) != "held" {
		t.Fatalf("the hub answered %q, want an open session", body)
	}
	return session
}

func TestLNK003ADeviceThatJoinedIsSeenByTheHubUnderItsOwnAddress(t *testing.T) {
	socket := testhub.Start(t)
	hub := startInvitingHubInFrontOf(t, socket.Socket)
	invite := hub.invite(t, "laptop")
	laptop, err := join(invite, identity.Device{Key: tunnel.NewPrivateKey()}, 30*time.Second)
	if err != nil {
		t.Fatal(err)
	}
	want, _ := hub.devices.AddressOf(laptop.Key.Public())
	if seen := seenUntil(t, startJoined(t, laptop)); seen != want {
		t.Errorf("the hub saw the device as %v, want %v", seen, want)
	}

	// A second use of the invite gives no device and no address.
	second := identity.Device{Key: tunnel.NewPrivateKey()}
	if _, err := join(invite, second, 5*time.Second); err == nil {
		t.Fatal("the invite worked twice")
	}
	second.Hub = laptop.Hub
	named := socket.Named()
	if seen, err := seenAs(startJoined(t, second), 4*time.Second); err == nil {
		t.Errorf("the second user of the invite reached the hub as %v", seen)
	}
	if socket.Named() != named || hub.devices.Count() != 1 {
		t.Error("the second use of the invite reached the hub's socket or its list")
	}
}

func TestLNK004ARevokedDevicesSessionEndsAtTheHubAndItReachesTheHubNoMore(t *testing.T) {
	socket := testhub.Start(t)
	hub := startInvitingHubInFrontOf(t, socket.Socket)
	laptop, desk := hub.joined(t), hub.joined(t)
	laptopLink, deskLink := startJoined(t, laptop), startJoined(t, desk)
	laptopAddress, deskAddress := seenUntil(t, laptopLink), seenUntil(t, deskLink)
	hold(t, laptopLink)
	hold(t, deskLink)

	if _, err := hub.link.Revoke(laptop.Key.Public()); err != nil {
		t.Fatal(err)
	}
	select {
	case ended := <-socket.Ended:
		if ended != laptopAddress {
			t.Errorf("the session of %v ended, want the revoked device %v", ended, laptopAddress)
		}
	case <-time.After(10 * time.Second):
		t.Fatal("the revoked device's session stayed open at the hub")
	}
	named := socket.Named()
	if seen, err := seenAs(laptopLink, 4*time.Second); err == nil {
		t.Errorf("the revoked device reached the hub as %v", seen)
	}
	if socket.Named() != named {
		t.Error("the hub's socket got a connection for the revoked device")
	}
	if seen, err := seenAs(deskLink, 10*time.Second); err != nil || seen != deskAddress {
		t.Errorf("the other device is seen as %v (%v), want %v", seen, err, deskAddress)
	}
	select {
	case ended := <-socket.Ended:
		t.Errorf("the session of %v ended; only the revoked device's must", ended)
	case <-time.After(time.Second):
	}
}

func TestLNK005TheHubSeesTheSameDeviceAfterAKeyRenewal(t *testing.T) {
	socket := testhub.Start(t)
	hub := startInvitingHubInFrontOf(t, socket.Socket)
	device := hub.joined(t)
	link, saved := hub.startRenewing(t, device)
	before := seenUntil(t, link)

	hub.clock.advance(151 * days)
	for deadline := time.Now().Add(30 * time.Second); saved.get().Key.Public() == device.Key.Public(); time.Sleep(100 * time.Millisecond) {
		if time.Now().After(deadline) {
			t.Fatal("the device did not renew its key")
		}
	}
	renewed := saved.get()
	hub.waitForKey(t, renewed.Key.Public(), "after the renewal")
	if after := seenUntil(t, link); after != before {
		t.Errorf("after the renewal the hub sees %v, want the same device %v", after, before)
	}
}

func TestLNK005AnExpiredKeyDoesNotReachTheHubsSocket(t *testing.T) {
	socket := testhub.Start(t)
	hub := startInvitingHubInFrontOf(t, socket.Socket)
	device := hub.joined(t)
	link := startJoined(t, device) // its own clock does not move: it does not renew
	seenUntil(t, link)

	hub.clock.advance(180 * days)
	time.Sleep(time.Second) // the sweep runs each 200 ms in this hub
	named := socket.Named()
	if seen, err := seenAs(link, 4*time.Second); err == nil {
		t.Errorf("a device with an expired key reached the hub as %v", seen)
	}
	if socket.Named() != named {
		t.Error("the hub's socket got a connection for an expired key")
	}
	if _, known := hub.devices.AddressOf(device.Key.Public()); known {
		t.Error("an expired key still has an address")
	}
}
