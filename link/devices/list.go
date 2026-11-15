// Package devices is the hub's list of permitted devices (LNK-002). A device
// is in the list by its key. list.go and renewal.go hold the rules; store.go
// keeps the list in the state directory.
package devices

import (
	"sort"
	"strings"
	"time"
	"unicode"
	"unicode/utf8"

	"github.com/TheArctesian/anima/link/tunnel"
)

// Refusal is a rule of the list that a request broke. Code is for programs
// and the text is for the operator.
type Refusal struct{ code, text string }

func (r *Refusal) Error() string { return r.text }

// Code names the rule for a program.
func (r *Refusal) Code() string { return r.code }

// The rules of the list.
var (
	ErrKnown   = &Refusal{"device_known", "this key is already permitted"}
	ErrBadName = &Refusal{"bad_name", "a device needs a name of 1 to 64 visible characters"}
	ErrNoKey   = &Refusal{"no_key", "a device needs a key"}
	ErrUnknown = &Refusal{"device_unknown", "this key is not in the list of permitted devices"}
	ErrNotDue  = &Refusal{"renewal_not_due", "a key can be renewed only in its last 30 days"}
)

const maxNameLength = 64

// KeyLife is how long a device key is permitted (LNK-005, as VPN-003).
const KeyLife = 180 * 24 * time.Hour

// Device is one permitted device.
type Device struct {
	Key tunnel.DeviceKey `json:"key"`
	// Name is what the operator calls the device.
	Name      string    `json:"name"`
	Permitted time.Time `json:"permitted"`
	// Expires is when Key stops being permitted.
	Expires time.Time `json:"expires"`
	// NextKey is the key that the device gave to replace Key. It is zero
	// when no renewal waits. See renewal.go.
	NextKey tunnel.DeviceKey `json:"nextKey,omitzero"`
}

// list is the set of permitted devices, by current key.
type list map[tunnel.DeviceKey]Device

// holder returns the device that has a key as its current or its next key.
func (l list) holder(key tunnel.DeviceKey) (Device, bool) {
	if device, known := l[key]; known {
		return device, true
	}
	for _, device := range l {
		if !key.IsZero() && device.NextKey == key {
			return device, true
		}
	}
	return Device{}, false
}

// permits reports whether a key can connect: it is the current or the next
// key of a device, and the device's current key has not expired.
func (l list) permits(key tunnel.DeviceKey, now time.Time) bool {
	device, known := l.holder(key)
	return known && now.Before(device.Expires)
}

// with returns the list with one more device. A device with no expiry time
// gets KeyLife from the time it was permitted.
func (l list) with(device Device) (list, error) {
	device.Name = strings.TrimSpace(device.Name)
	switch {
	case device.Key.IsZero():
		return nil, ErrNoKey
	case ValidName(device.Name) != nil:
		return nil, ErrBadName
	}
	if _, known := l.holder(device.Key); known {
		return nil, ErrKnown
	}
	if device.Expires.IsZero() {
		device.Expires = device.Permitted.Add(KeyLife)
	}
	next := l.copy()
	next[device.Key] = device
	return next, nil
}

// without returns the list with one device less. The key is the device's
// current or next key: both leave the list.
func (l list) without(key tunnel.DeviceKey) (list, Device, error) {
	device, known := l.holder(key)
	if !known {
		return nil, Device{}, ErrUnknown
	}
	next := l.copy()
	delete(next, device.Key)
	return next, device, nil
}

func (l list) copy() list {
	next := make(list, len(l)+1)
	for key, device := range l {
		next[key] = device
	}
	return next
}

// ValidName returns ErrBadName unless a name is 1 to 64 visible characters
// with no space at its ends. A control or format character could disguise a
// name on the operator's terminal.
func ValidName(name string) error {
	if name == "" || name != strings.TrimSpace(name) || !utf8.ValidString(name) || utf8.RuneCountInString(name) > maxNameLength {
		return ErrBadName
	}
	for _, r := range name {
		if unicode.IsControl(r) || unicode.Is(unicode.Cf, r) {
			return ErrBadName
		}
	}
	return nil
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
