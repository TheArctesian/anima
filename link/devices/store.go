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

// Permits reports whether a key can connect now.
func (s *Store) Permits(key tunnel.DeviceKey) bool {
	s.mu.RLock()
	defer s.mu.RUnlock()
	return s.current.permits(key)
}

// All returns the permitted devices, oldest first.
func (s *Store) All() []Device {
	s.mu.RLock()
	defer s.mu.RUnlock()
	return s.current.sorted()
}
