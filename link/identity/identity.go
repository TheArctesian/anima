// Package identity holds what a hub or a device must keep between starts to
// stay the same hub or device (LNK-001).
package identity

import (
	"errors"

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
	Key          tunnel.PrivateKey   `json:"key"`
	PresharedKey tunnel.PresharedKey `json:"presharedKey"`
	// Relay is zero until the first start has chosen a relay.
	Relay tunnel.Relay `json:"relay"`
}

// LoadHub reads the hub's identity. At the first call it makes the keys and
// saves them.
func LoadHub(dir *statedir.Dir) (Hub, error) {
	var hub Hub
	err := dir.Read(hubFile, &hub)
	if errors.Is(err, statedir.ErrAbsent) {
		hub = Hub{Key: tunnel.NewPrivateKey(), PresharedKey: tunnel.NewPresharedKey()}
		return hub, hub.Save(dir)
	}
	return hub, err
}

// Save writes the hub's identity. A hub calls it when its relay is chosen
// or changed.
func (h Hub) Save(dir *statedir.Dir) error { return dir.Write(hubFile, h) }

// Device is the saved identity of a device.
type Device struct {
	Key tunnel.PrivateKey `json:"key"`
	// Hub is the address of the device's hub. It is empty before enrolment.
	Hub tunnel.Address `json:"hub,omitempty"`
}

// LoadDevice reads the device's identity. At the first call it makes the
// key and saves it.
func LoadDevice(dir *statedir.Dir) (Device, error) {
	var device Device
	err := dir.Read(deviceFile, &device)
	if errors.Is(err, statedir.ErrAbsent) {
		device = Device{Key: tunnel.NewPrivateKey()}
		return device, device.Save(dir)
	}
	return device, err
}

// Save writes the device's identity.
func (d Device) Save(dir *statedir.Dir) error { return dir.Write(deviceFile, d) }
