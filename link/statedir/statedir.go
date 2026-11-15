// Package statedir keeps the files of one anima-link in a directory that
// only its owner can read (LNK-008). One process uses a directory at a time.
package statedir

import (
	"encoding/json"
	"errors"
	"fmt"
	"io/fs"
	"os"
	"path/filepath"
	"syscall"
)

// Errors of this package. None of them holds the content of a file.
var (
	// ErrInUse means that another anima-link has the directory.
	ErrInUse = errors.New("statedir: another anima-link uses this directory")
	// ErrOpen means that other users can reach the directory or a file.
	ErrOpen = errors.New("statedir: other users can read this path; set owner-only access")
	// ErrDamaged means that a file cannot be read as what it must hold.
	ErrDamaged = errors.New("statedir: the file is damaged")
	// ErrAbsent means that the file was never written.
	ErrAbsent = errors.New("statedir: no such file")
)

const lockName = "lock"

// Dir is an open state directory. Close it to let another process use it.
type Dir struct {
	path string
	lock *os.File
}

// Open makes the directory if it is absent, checks that only its owner can
// reach it, and takes it for this process.
func Open(path string) (*Dir, error) {
	if err := os.MkdirAll(path, 0o700); err != nil {
		return nil, err
	}
	if err := ownerOnly(path); err != nil {
		return nil, err
	}
	lock, err := os.OpenFile(filepath.Join(path, lockName), os.O_CREATE|os.O_RDWR, 0o600)
	if err != nil {
		return nil, err
	}
	if err := syscall.Flock(int(lock.Fd()), syscall.LOCK_EX|syscall.LOCK_NB); err != nil {
		lock.Close()
		return nil, fmt.Errorf("%w: %s", ErrInUse, path)
	}
	return &Dir{path: path, lock: lock}, nil
}

// Close gives the directory back.
func (d *Dir) Close() error { return d.lock.Close() }

// Read fills value from the JSON file name. It returns ErrAbsent when the
// file was never written.
func (d *Dir) Read(name string, value any) error {
	path := filepath.Join(d.path, name)
	text, err := os.ReadFile(path)
	if errors.Is(err, fs.ErrNotExist) {
		return fmt.Errorf("%w: %s", ErrAbsent, name)
	}
	if err != nil {
		return err
	}
	if err := ownerOnly(path); err != nil {
		return err
	}
	if err := json.Unmarshal(text, value); err != nil {
		// The cause can quote the file, and the file can hold a key.
		return fmt.Errorf("%w: %s", ErrDamaged, name)
	}
	return nil
}

// Write replaces the JSON file name with value. A reader sees the old file
// or the new file, never a part of one.
func (d *Dir) Write(name string, value any) error {
	text, err := json.Marshal(value)
	if err != nil {
		return err
	}
	temporary, err := os.CreateTemp(d.path, name+".*")
	if err != nil {
		return err
	}
	defer os.Remove(temporary.Name())
	if _, err := temporary.Write(text); err != nil {
		temporary.Close()
		return err
	}
	if err := temporary.Sync(); err != nil {
		temporary.Close()
		return err
	}
	if err := temporary.Close(); err != nil {
		return err
	}
	return os.Rename(temporary.Name(), filepath.Join(d.path, name))
}

// ownerOnly refuses a path that its group or other users can reach, or that
// another user owns.
func ownerOnly(path string) error {
	stat, err := os.Stat(path)
	if err != nil {
		return err
	}
	owner, known := stat.Sys().(*syscall.Stat_t)
	if stat.Mode().Perm()&0o077 != 0 || (known && int(owner.Uid) != os.Getuid()) {
		return fmt.Errorf("%w: %s", ErrOpen, path)
	}
	return nil
}
