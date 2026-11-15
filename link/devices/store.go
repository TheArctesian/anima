package devices

import (
	"errors"
	"net/netip"
	"sync"
	"time"

	"github.com/TheArctesian/anima/link/statedir"
	"github.com/TheArctesian/anima/link/tunnel"
)

const fileName = "devices.json"

// Store is the list of permitted devices of a running hub link. Each change
// is on disk before the call returns.
type Store struct {
	dir *statedir.Dir
	now func() time.Time

	mu      sync.RWMutex
	current list
}

// Open reads the list from the state directory. A directory with no list
// has no permitted device: it refuses every key. now is the clock: a key
// expires by it (LNK-005).
func Open(dir *statedir.Dir, now func() time.Time) (*Store, error) {
	var saved []Device
	if err := dir.Read(fileName, &saved); err != nil && !errors.Is(err, statedir.ErrAbsent) {
		return nil, err
	}
	current := list{}
	for _, device := range saved {
		// A list that was written before devices had addresses.
		if !linkRange.Contains(device.Address) {
			device.Address = addressOf(device.Key)
		}
		current[device.Key] = device
	}
	return &Store{dir: dir, now: now, current: current}, nil
}

// Permit adds a device to the list. A device with no Permitted time gets
// the time of the call.
func (s *Store) Permit(device Device) (Device, error) {
	s.mu.Lock()
	defer s.mu.Unlock()
	if device.Permitted.IsZero() {
		device.Permitted = s.now().UTC()
	}
	next, err := s.current.with(device)
	if err != nil {
		return Device{}, err
	}
	return next[device.Key], s.save(next)
}

// Revoke removes a device from the list (LNK-004), with its current key and
// its next key. It returns the device, so that the hub link can disconnect
// both keys: see hublink.Disconnect.
func (s *Store) Revoke(key tunnel.DeviceKey) (Device, error) {
	s.mu.Lock()
	defer s.mu.Unlock()
	next, device, err := s.current.without(key)
	if err != nil {
		return Device{}, err
	}
	return device, s.save(next)
}

// Renew records the next key of the device that is connected with current
// (LNK-005). The device can then connect with either key.
func (s *Store) Renew(current, next tunnel.DeviceKey) (Device, error) {
	s.mu.Lock()
	defer s.mu.Unlock()
	updated, device, err := s.current.renewing(current, next, s.now())
	if err != nil {
		return Device{}, err
	}
	return device, s.save(updated)
}

// Promote makes a next key the current key of its device. The caller has
// a stream from the next key, which proves that the device holds it. The
// result names the key that was replaced, for the hub link to disconnect.
// For a key that is not a next key it returns nil and changes nothing.
func (s *Store) Promote(next tunnel.DeviceKey) (*Promotion, error) {
	s.mu.Lock()
	defer s.mu.Unlock()
	updated, promotion, promoted := s.current.promoted(next, s.now().UTC())
	if !promoted {
		return nil, nil
	}
	return &promotion, s.save(updated)
}

// Permits reports whether a key can connect now: it is the current or the
// next key of a device in the list, and the device's key has not expired.
// The hub link asks when a device connects and for each new stream. A device
// that is connected stays connected when it leaves the list or expires,
// until the hub link disconnects it (LNK-004).
func (s *Store) Permits(key tunnel.DeviceKey) bool {
	_, permitted := s.AddressOf(key)
	return permitted
}

// AddressOf returns the link address of the device that a key can connect
// as now. It reports false for a key that Permits refuses.
func (s *Store) AddressOf(key tunnel.DeviceKey) (netip.Addr, bool) {
	s.mu.RLock()
	defer s.mu.RUnlock()
	device, permitted := s.current.permitted(key, s.now())
	return device.Address, permitted
}

// Count is the number of devices in the list.
func (s *Store) Count() int {
	s.mu.RLock()
	defer s.mu.RUnlock()
	return len(s.current)
}

// All returns the devices in the list, oldest first.
func (s *Store) All() []Device {
	s.mu.RLock()
	defer s.mu.RUnlock()
	return s.current.sorted()
}

// save writes the list and, when it is on disk, makes it the current list.
func (s *Store) save(next list) error {
	if err := s.dir.Write(fileName, next.sorted()); err != nil {
		return err
	}
	s.current = next
	return nil
}
