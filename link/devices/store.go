package devices

import (
	"errors"
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

// Promote makes a next key the current key of its device, at the first time
// that the next key connects. It returns the key that was replaced, for the
// hub link to disconnect. It reports false, and changes nothing, for a key
// that is not a next key.
func (s *Store) Promote(next tunnel.DeviceKey) (old tunnel.DeviceKey, promoted bool, err error) {
	s.mu.Lock()
	defer s.mu.Unlock()
	updated, old, promoted := s.current.promoted(next, s.now().UTC())
	if !promoted {
		return tunnel.DeviceKey{}, false, nil
	}
	return old, true, s.save(updated)
}

// Permits reports whether a key can connect now: it is the current or the
// next key of a device in the list, and the device's key has not expired.
// The hub link asks when a device connects and for each new stream. A device
// that is connected stays connected when it leaves the list or expires,
// until the hub link disconnects it (LNK-004).
func (s *Store) Permits(key tunnel.DeviceKey) bool {
	s.mu.RLock()
	defer s.mu.RUnlock()
	return s.current.permits(key, s.now())
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
