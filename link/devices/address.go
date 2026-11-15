package devices

import (
	"crypto/sha256"
	"net/netip"

	"github.com/TheArctesian/anima/link/tunnel"
)

// Each device in the list has one link address. The hub link gives it when
// it permits the device's key, and names the device to the hub by it: the
// hub's rules, its database and its audit log name a client by an address.
//
// The address is not a network address and no packet is sent to it. It is
// the link range and 80 bits of a hash of the key that the device joined
// with. A device keeps its address when it renews its key (LNK-005), so
// what the hub bound to the device stays bound. The address leaves the list
// with the device (LNK-004).

// linkRange holds every link address: a unique local IPv6 range whose
// global ID is the text "ANIMA". The hub accepts no other address from the
// link, so the two programs must agree on it.
var linkRange = netip.MustParsePrefix("fd41:4e49:4d41::/48")

// LinkRange is the range that holds every link address.
func LinkRange() netip.Prefix { return linkRange }

const addressLabel = "anima-link device address 1\n"

// addressOf is the link address that a key gets when it joins.
func addressOf(key tunnel.DeviceKey) netip.Addr {
	text, _ := key.MarshalText() // never fails for a key
	sum := sha256.Sum256(append([]byte(addressLabel), text...))
	raw := linkRange.Addr().As16()
	copy(raw[linkRange.Bits()/8:], sum[:])
	return netip.AddrFrom16(raw)
}

// addressTaken reports whether a device in the list has an address.
func (l list) addressTaken(address netip.Addr) bool {
	for _, device := range l {
		if device.Address == address {
			return true
		}
	}
	return false
}
