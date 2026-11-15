// Package devices is the hub's list of permitted devices (LNK-002). A device
// is in the list by its key. This file holds the rules; store.go keeps the
// list in the state directory.
package devices

import (
	"errors"
	"sort"
	"strings"
	"time"

	"github.com/TheArctesian/anima/link/tunnel"
)

// Errors of this package.
var (
	ErrKnown   = errors.New("devices: this key is already permitted")
	ErrBadName = errors.New("devices: a device needs a name of 1 to 64 characters")
	ErrNoKey   = errors.New("devices: a device needs a key")
)

const maxNameLength = 64

// Device is one permitted device.
type Device struct {
	Key tunnel.DeviceKey `json:"key"`
	// Name is what the operator calls the device.
	Name      string    `json:"name"`
	Permitted time.Time `json:"permitted"`
}

// list is the set of permitted devices, by key.
type list map[tunnel.DeviceKey]Device

// with returns the list with one more device.
func (l list) with(device Device) (list, error) {
	device.Name = strings.TrimSpace(device.Name)
	switch {
	case device.Key.IsZero():
		return nil, ErrNoKey
	case device.Name == "" || len([]rune(device.Name)) > maxNameLength:
		return nil, ErrBadName
	}
	if _, known := l[device.Key]; known {
		return nil, ErrKnown
	}
	next := make(list, len(l)+1)
	for key, each := range l {
		next[key] = each
	}
	next[device.Key] = device
	return next, nil
}

// permits reports whether a key is in the list.
func (l list) permits(key tunnel.DeviceKey) bool {
	_, known := l[key]
	return known
}

// sorted returns the devices, oldest first.
func (l list) sorted() []Device {
	all := make([]Device, 0, len(l))
	for _, device := range l {
		all = append(all, device)
	}
	sort.Slice(all, func(i, j int) bool {
		if !all[i].Permitted.Equal(all[j].Permitted) {
			return all[i].Permitted.Before(all[j].Permitted)
		}
		return all[i].Name < all[j].Name
	})
	return all
}
