// Package statedir keeps the files of one anima-link in a directory that
// only its owner can read (LNK-008). One process uses a directory at a time.
package statedir

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"io/fs"
	"net"
	"os"
	"path/filepath"
	"syscall"
	"time"
)

// Errors of this package. None of them holds the content of a file.
var (
	// ErrInUse means that another anima-link has the directory.
	ErrInUse = errors.New("statedir: another anima-link uses this directory")
	// ErrExposed means that other users can reach the directory or a file,
	// or can replace it.
	ErrExposed = errors.New("statedir: other users can reach this path; set owner-only access")
	// ErrDamaged means that a file cannot be read as what it must hold.
	ErrDamaged = errors.New("statedir: the file is damaged")
	// ErrAbsent means that the file was never written.
	ErrAbsent = errors.New("statedir: no such file")
	// ErrLongPath means that the directory's path leaves no room for a
	// socket name. The system limit for a socket path is about 100 bytes.
	ErrLongPath = errors.New("statedir: the path is too long for a socket; use a shorter state directory")
)

// maxSocketPath is the smaller of the macOS and Linux limits, less the end byte.
const maxSocketPath = 103

const lockName = "lock"

// Dir is an open state directory. Close it to let another process use it.
type Dir struct {
	path string
	lock *os.File
}

// Open makes the directory if it is absent, checks that only its owner can
// reach it, and takes it for this process.
func Open(path string) (*Dir, error) {
	path, err := filepath.Abs(path)
	if err != nil {
		return nil, err
	}
	if err := os.MkdirAll(path, 0o700); err != nil {
		return nil, fmt.Errorf("statedir: make the directory: %w", err)
	}
	if err := private(path); err != nil {
		return nil, err
	}
	// Only now: the directory is this user's own and nobody else reaches it.
	if err := dropInheritedAccess(path); err != nil {
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

// Beside opens the directory of an anima-link that may run now, without
// taking it: for a file that a second process changes, such as the open
// ports of a device. A file that Write replaces is read whole, the old or
// the new one. The directory must exist and pass the same test as in Open.
func Beside(path string) (*Dir, error) {
	path, err := filepath.Abs(path)
	if err != nil {
		return nil, err
	}
	if err := private(path); err != nil {
		return nil, err
	}
	return &Dir{path: path}, nil
}

// Close gives the directory back.
func (d *Dir) Close() error {
	if d.lock == nil {
		return nil
	}
	return d.lock.Close()
}

// Read fills value from the JSON file name. It returns ErrAbsent when the
// file was never written.
func (d *Dir) Read(name string, value any) error {
	file, err := os.OpenFile(filepath.Join(d.path, name), os.O_RDONLY|syscall.O_NOFOLLOW|syscall.O_NONBLOCK, 0)
	if errors.Is(err, fs.ErrNotExist) {
		return fmt.Errorf("%w: %s", ErrAbsent, name)
	}
	if err != nil {
		return fmt.Errorf("%w: %s", ErrExposed, name)
	}
	defer file.Close()
	// The check and the read use the same open file, so the file cannot be
	// replaced between them.
	stat, err := file.Stat()
	if err != nil {
		return err
	}
	if !stat.Mode().IsRegular() || !ownerOnly(stat) {
		return fmt.Errorf("%w: %s", ErrExposed, name)
	}
	text, err := io.ReadAll(file)
	if err != nil {
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
	if err := os.Rename(temporary.Name(), filepath.Join(d.path, name)); err != nil {
		return err
	}
	return d.sync()
}

// sync makes a rename in the directory last through a power loss. Without
// it a hub could lose its new identity file and come back as another hub.
func (d *Dir) sync() error {
	directory, err := os.Open(d.path)
	if err != nil {
		return err
	}
	defer directory.Close()
	return directory.Sync()
}

// Listen makes the Unix socket name in the directory. Only the owner can
// connect to it. A socket that an earlier process left is replaced: this
// process holds the directory, so no other process serves it.
func (d *Dir) Listen(name string) (net.Listener, error) {
	path := filepath.Join(d.path, name)
	if len(path) > maxSocketPath {
		return nil, ErrLongPath
	}
	if err := os.Remove(path); err != nil && !errors.Is(err, fs.ErrNotExist) {
		return nil, err
	}
	listener, err := net.Listen("unix", path)
	if err != nil {
		return nil, err
	}
	if err := os.Chmod(path, 0o600); err != nil {
		listener.Close()
		return nil, err
	}
	return listener, nil
}

// Dial connects to the socket name of the state directory at path. The
// caller is a second process: it does not take the directory.
func Dial(ctx context.Context, path, name string) (net.Conn, error) {
	var dialer net.Dialer
	return dialer.DialContext(ctx, "unix", filepath.Join(path, name))
}

// DialOwned connects to a Unix socket of this user's other program, for
// example the hub's socket for the link. It refuses a socket that another
// user owns or can replace: what is sent there would reach that user. The
// socket's folder must pass the same test as a state directory. The socket
// is checked and then opened by its path: only this user or root can change
// what the path leads to between the two.
func DialOwned(path string, limit time.Duration) (net.Conn, error) {
	if err := private(filepath.Dir(path)); err != nil {
		return nil, err
	}
	stat, err := os.Lstat(path)
	if err != nil {
		return nil, err
	}
	if stat.Mode().Type() != fs.ModeSocket || !ownerOnly(stat) {
		return nil, fmt.Errorf("%w: %s", ErrExposed, path)
	}
	return net.DialTimeout("unix", path, limit)
}

// private refuses a directory that is a symbolic link, that other users can
// reach, or that another user can replace: a folder above it that another
// user owns, or that others can write with no sticky bit.
func private(path string) error {
	stat, err := os.Lstat(path)
	if err != nil {
		return err
	}
	if !stat.IsDir() || !ownerOnly(stat) {
		return fmt.Errorf("%w: %s", ErrExposed, path)
	}
	return foldersAbove(path)
}

func ownedByRootOrThisUser(stat fs.FileInfo) bool {
	owner, known := stat.Sys().(*syscall.Stat_t)
	return !known || owner.Uid == 0 || int(owner.Uid) == os.Getuid()
}

// ownerOnly reports whether only this user can reach the file.
func ownerOnly(stat fs.FileInfo) bool {
	owner, known := stat.Sys().(*syscall.Stat_t)
	return stat.Mode().Perm()&0o077 == 0 && (!known || int(owner.Uid) == os.Getuid())
}
