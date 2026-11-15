package invites

import (
	"errors"
	"io"
	"sync"
	"time"

	"github.com/TheArctesian/anima/link/devices"
	"github.com/TheArctesian/anima/link/statedir"
	"github.com/TheArctesian/anima/link/tunnel"
)

const fileName = "invites.json"

// Store is the open invites of a running hub link. Each change is on disk
// before the call returns, so an invite that was used stays used after a
// restart.
type Store struct {
	dir *statedir.Dir

	mu      sync.Mutex
	current book
}

// Open reads the open invites from the state directory.
func Open(dir *statedir.Dir) (*Store, error) {
	var saved []Record
	if err := dir.Read(fileName, &saved); err != nil && !errors.Is(err, statedir.ErrAbsent) {
		return nil, err
	}
	current := book{}
	for _, record := range saved {
		current[record.Key] = record
	}
	return &Store{dir: dir, current: current}, nil
}

// Make makes an invite for a device with the given name. The invite is the
// return value only: the store keeps the public half of its temporary key.
func (s *Store) Make(hub tunnel.Address, name string, now time.Time, random io.Reader) (Invite, Record, error) {
	if err := devices.ValidName(name); err != nil {
		return Invite{}, Record{}, err
	}
	invite, err := newInvite(hub, random)
	if err != nil {
		return Invite{}, Record{}, err
	}
	record := Record{Key: invite.TemporaryKey().Public(), Name: name, Expires: now.Add(Life)}
	s.mu.Lock()
	defer s.mu.Unlock()
	if err := s.save(s.current.with(record, now)); err != nil {
		return Invite{}, Record{}, err
	}
	return invite, record, nil
}

// Admits reports whether a key is the temporary key of an open invite.
func (s *Store) Admits(key tunnel.DeviceKey, now time.Time) bool {
	s.mu.Lock()
	defer s.mu.Unlock()
	return s.current.open(key, now)
}

// Use ends an open invite and returns its record. Only the first call for
// an invite succeeds, also when what the caller does next fails (LNK-003).
// Cancelling an invite is the same act.
func (s *Store) Use(key tunnel.DeviceKey, now time.Time) (Record, error) {
	s.mu.Lock()
	defer s.mu.Unlock()
	next, record, err := s.current.without(key, now)
	if err != nil {
		return Record{}, err
	}
	return record, s.save(next)
}

// All returns the invites that are open now, the first to expire first.
func (s *Store) All(now time.Time) []Record {
	s.mu.Lock()
	defer s.mu.Unlock()
	return s.current.sorted(now)
}

func (s *Store) save(next book) error {
	if err := s.dir.Write(fileName, next.sorted(time.Time{})); err != nil {
		return err
	}
	s.current = next
	return nil
}
