// Package devices is the hub's list of permitted devices (LNK-002). A device
// is in the list by its key. This file holds the rules; store.go keeps the
// list in the state directory.
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
type Refusal struct {
	Code string
	text string
}

func (r *Refusal) Error() string { return r.text }

// The rules of the list.
var (
	ErrKnown   = &Refusal{"device_known", "this key is already permitted"}
	ErrBadName = &Refusal{"bad_name", "a device needs a name of 1 to 64 visible characters"}
	ErrNoKey   = &Refusal{"no_key", "a device needs a key"}
	ErrUnknown = &Refusal{"device_unknown", "this key is not in the list of permitted devices"}
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
	case !plainName(device.Name):
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

// without returns the list with one device less.
func (l list) without(key tunnel.DeviceKey) (list, error) {
	if _, known := l[key]; !known {
		return nil, ErrUnknown
	}
	next := make(list, len(l))
	for each, device := range l {
		if each != key {
			next[each] = device
		}
	}
	return next, nil
}

// plainName reports whether a name is 1 to 64 visible characters. A control
// or format character could disguise a name on the operator's terminal.
func plainName(name string) bool {
	if name == "" || !utf8.ValidString(name) || utf8.RuneCountInString(name) > maxNameLength {
		return false
	}
	for _, r := range name {
		if unicode.IsControl(r) || unicode.Is(unicode.Cf, r) {
			return false
		}
	}
	return true
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
