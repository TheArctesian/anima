// Package identity holds what a hub or a device must keep between starts to
// stay the same hub or device (LNK-001).
package identity

import (
	"errors"
	"fmt"
	"time"

	"github.com/TheArctesian/anima/link/statedir"
	"github.com/TheArctesian/anima/link/tunnel"
)

const (
	hubFile    = "hub.json"
	deviceFile = "device.json"
)

// Hub is the saved identity of a hub. The key, the pre-shared key and the
// relay together give the hub address, so all three are kept.
type Hub struct {
	Key          tunnel.PrivateKey
	PresharedKey tunnel.PresharedKey
	// Relay is zero until the first start has chosen a relay.
	Relay tunnel.Relay
}

// savedHub is the file form of Hub. Only this form holds the secrets as text.
type savedHub struct {
	Key          string       `json:"key"`
	PresharedKey string       `json:"presharedKey"`
	Relay        tunnel.Relay `json:"relay"`
}

// LoadHub reads the hub's identity. At the first call it makes the keys and
// saves them.
func LoadHub(dir *statedir.Dir) (Hub, error) {
	var saved savedHub
	err := dir.Read(hubFile, &saved)
	if errors.Is(err, statedir.ErrAbsent) {
		hub := Hub{Key: tunnel.NewPrivateKey(), PresharedKey: tunnel.NewPresharedKey()}
		return hub, hub.Save(dir)
	}
	if err != nil {
		return Hub{}, err
	}
	key, keyErr := tunnel.ParsePrivateKey(saved.Key)
	psk, pskErr := tunnel.ParsePresharedKey(saved.PresharedKey)
	if keyErr != nil || pskErr != nil {
		return Hub{}, fmt.Errorf("%w: %s", statedir.ErrDamaged, hubFile)
	}
	return Hub{Key: key, PresharedKey: psk, Relay: saved.Relay}, nil
}

// Save writes the hub's identity. A hub calls it when its relay is chosen
// or changed.
func (h Hub) Save(dir *statedir.Dir) error {
	return dir.Write(hubFile, savedHub{h.Key.Reveal(), h.PresharedKey.Reveal(), h.Relay})
}

// Device is the saved identity of a device.
type Device struct {
	Key tunnel.PrivateKey
	// Hub is the address of the device's hub. It is zero before enrolment.
	Hub tunnel.Address
	// Expires is when the hub stops permitting Key (LNK-005).
	Expires time.Time
	// NextKey is the key that replaces Key. It is set from the moment the
	// device starts a renewal until the hub has accepted it.
	NextKey tunnel.PrivateKey
}

// savedDevice is the file form of Device.
type savedDevice struct {
	Key     string    `json:"key"`
	Hub     string    `json:"hub,omitempty"`
	Expires time.Time `json:"expires,omitzero"`
	NextKey string    `json:"nextKey,omitempty"`
}

// LoadDevice reads the device's identity. At the first call it makes the
// key and saves it.
func LoadDevice(dir *statedir.Dir) (Device, error) {
	var saved savedDevice
	err := dir.Read(deviceFile, &saved)
	if errors.Is(err, statedir.ErrAbsent) {
		device := Device{Key: tunnel.NewPrivateKey()}
		return device, device.Save(dir)
	}
	if err != nil {
		return Device{}, err
	}
	device := Device{Expires: saved.Expires}
	if device.Key, err = tunnel.ParsePrivateKey(saved.Key); err != nil {
		return Device{}, fmt.Errorf("%w: %s", statedir.ErrDamaged, deviceFile)
	}
	if saved.NextKey != "" {
		if device.NextKey, err = tunnel.ParsePrivateKey(saved.NextKey); err != nil {
			return Device{}, fmt.Errorf("%w: %s", statedir.ErrDamaged, deviceFile)
		}
	}
	if saved.Hub != "" {
		if device.Hub, err = tunnel.ParseAddress(saved.Hub); err != nil {
			return Device{}, fmt.Errorf("%w: %s", statedir.ErrDamaged, deviceFile)
		}
	}
	return device, nil
}

// Save writes the device's identity.
func (d Device) Save(dir *statedir.Dir) error {
	saved := savedDevice{Key: d.Key.Reveal(), Hub: d.Hub.Reveal(), Expires: d.Expires}
	if !d.NextKey.IsZero() {
		saved.NextKey = d.NextKey.Reveal()
	}
	return dir.Write(deviceFile, saved)
}
