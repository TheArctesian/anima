package devices

import (
	"net/netip"

	"github.com/TheArctesian/anima/link/tunnel"
)

// The hub keeps what it bound to a device's address for longer than the
// device is in the list: a builder, a session, the audit log. A key or an
// address that has left the list must therefore never come back, or the
// holder of an old key would get what the hub bound to the revoked device
// (LNK-004). retired holds each key and each address that has left the list.
// It only grows: one key for each revocation and each renewal.

const retiredFileName = "retired.json"

// retired is the keys and addresses that can never be permitted again.
type retired struct {
	Keys      []tunnel.DeviceKey `json:"keys"`
	Addresses []netip.Addr       `json:"addresses"`
}

func (r retired) hasKey(key tunnel.DeviceKey) bool {
	for _, gone := range r.Keys {
		if gone == key {
			return true
		}
	}
	return false
}

func (r retired) hasAddress(address netip.Addr) bool {
	for _, gone := range r.Addresses {
		if gone == address {
			return true
		}
	}
	return false
}

// refuses reports whether a key cannot be permitted: it has left the list,
// or the address it would get has.
func (r retired) refuses(key tunnel.DeviceKey) bool {
	return r.hasKey(key) || r.hasAddress(addressOf(key))
}

// withKeys returns the set with more keys. Zero keys and known keys are
// left out.
func (r retired) withKeys(keys ...tunnel.DeviceKey) retired {
	next := retired{Keys: append([]tunnel.DeviceKey(nil), r.Keys...), Addresses: r.Addresses}
	for _, key := range keys {
		if !key.IsZero() && !next.hasKey(key) {
			next.Keys = append(next.Keys, key)
		}
	}
	return next
}

// withDevice returns the set with both keys and the address of a device
// that leaves the list.
func (r retired) withDevice(device Device) retired {
	next := r.withKeys(device.Key, device.NextKey)
	if !next.hasAddress(device.Address) {
		next.Addresses = append(append([]netip.Addr(nil), next.Addresses...), device.Address)
	}
	return next
}
