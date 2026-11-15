package devices

import (
	"errors"
	"sync"

	"github.com/TheArctesian/anima/link/statedir"
	"github.com/TheArctesian/anima/link/tunnel"
)

const fileName = "devices.json"

// Store is the list of permitted devices of a running hub link. Each change
// is on disk before the call returns.
type Store struct {
	dir *statedir.Dir

	mu      sync.RWMutex
	current list
}

// Open reads the list from the state directory. A directory with no list
// has no permitted device: it refuses every key.
func Open(dir *statedir.Dir) (*Store, error) {
	var saved []Device
	if err := dir.Read(fileName, &saved); err != nil && !errors.Is(err, statedir.ErrAbsent) {
		return nil, err
	}
	current := list{}
	for _, device := range saved {
		current[device.Key] = device
	}
	return &Store{dir: dir, current: current}, nil
}

// Permit adds a device to the list.
func (s *Store) Permit(device Device) error {
	s.mu.Lock()
	defer s.mu.Unlock()
	next, err := s.current.with(device)
	if err != nil {
		return err
	}
	if err := s.dir.Write(fileName, next.sorted()); err != nil {
		return err
	}
	s.current = next
	return nil
}

// Revoke removes a device from the list (LNK-004). The key cannot connect
// again. The hub link must then disconnect the device: see hublink.Disconnect.
func (s *Store) Revoke(key tunnel.DeviceKey) error {
	s.mu.Lock()
	defer s.mu.Unlock()
	next, err := s.current.without(key)
	if err != nil {
		return err
	}
	if err := s.dir.Write(fileName, next.sorted()); err != nil {
		return err
	}
	s.current = next
	return nil
}

// Permits reports whether a key is in the list. The hub link asks when a
// device connects. A device that is connected stays connected when its key
// leaves the list, until the hub link disconnects it (LNK-004).
func (s *Store) Permits(key tunnel.DeviceKey) bool {
	s.mu.RLock()
	defer s.mu.RUnlock()
	return s.current.permits(key)
}

// Count is the number of permitted devices.
func (s *Store) Count() int {
	s.mu.RLock()
	defer s.mu.RUnlock()
	return len(s.current)
}

// All returns the permitted devices, oldest first.
func (s *Store) All() []Device {
	s.mu.RLock()
	defer s.mu.RUnlock()
	return s.current.sorted()
}
