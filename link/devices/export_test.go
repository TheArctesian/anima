package devices

import (
	"net/netip"

	"github.com/TheArctesian/anima/link/tunnel"
)

// LinkRange is the range that holds every link address.
func LinkRange() netip.Prefix { return linkRange }

// PermitBesideItsOwnAddress tries to permit a key in a list where another
// device already has that key's link address. Two real keys with one
// address cannot be made for a test.
func PermitBesideItsOwnAddress(s *Store, key tunnel.DeviceKey) error {
	other := tunnel.NewPrivateKey().Public()
	taken := list{other: Device{Key: other, Name: "other", Address: addressOf(key)}}
	_, err := taken.with(Device{Key: key, Name: "new", Permitted: s.now()})
	return err
}
