package hublink

import (
	"encoding/json"
	"net"
	"time"

	"github.com/TheArctesian/anima/link/invites"
	"github.com/TheArctesian/anima/link/tunnel"
)

// A relay change gives the hub a new address, because the address holds the
// relay (LNK-007). The change has two steps, so that a connected device
// needs no new invite:
//
//  1. Announce: the hub link tells each permitted device the address that
//     it will have. A device reads it on the notice port and keeps it.
//  2. The hub link starts again on the new relay. A device that gets no
//     answer at the old address tries the address that it kept.
//
// A device that was off during step 1 does not know the new address: it
// needs a new invite, or the domain name of the hub.

// Announce starts step 1 for a relay, and returns the address that the hub
// will have on it.
func (l *Link) Announce(next tunnel.Relay) tunnel.Address {
	address := tunnel.AddressOn(l.identity.Key, l.identity.PresharedKey, next)
	l.next.Store(&address)
	return address
}

// notify serves one stream of the notice port: it writes the next address,
// if there is one, to a permitted device.
func (l *Link) notify(stream net.Conn) {
	defer stream.Close()
	device, known := l.hub.DeviceOf(stream.RemoteAddr())
	if !known || !l.devices.Permits(device) {
		return
	}
	var notice invites.Notice
	if next := l.next.Load(); next != nil {
		notice.NextHub = next.Reveal()
	}
	_ = stream.SetDeadline(time.Now().Add(enrolLimit))
	_ = json.NewEncoder(stream).Encode(notice)
}
