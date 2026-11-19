package statedir

import (
	"errors"
	"fmt"
	"io/fs"
	"net"
	"os"
	"path/filepath"
)

// ListenOwned makes a Unix socket at path for this user's other program,
// for example the socket of a device link for the metrics daemon (K8
// Section 1). Only the owner can connect to it. The socket's folder must
// pass the same test as a state directory, so no other user can put a
// socket of their own at the path. A socket that an earlier process left is
// replaced; a path that holds a thing that is not a socket is refused.
func ListenOwned(path string) (net.Listener, error) {
	path, err := filepath.Abs(path)
	if err != nil {
		return nil, err
	}
	if len(path) > maxSocketPath {
		return nil, ErrLongPath
	}
	if err := private(filepath.Dir(path)); err != nil {
		return nil, err
	}
	stat, err := os.Lstat(path)
	switch {
	case errors.Is(err, fs.ErrNotExist):
	case err != nil:
		return nil, err
	case stat.Mode().Type() != fs.ModeSocket || !ownerOnly(stat):
		return nil, fmt.Errorf("%w: %s", ErrExposed, path)
	default:
		if err := os.Remove(path); err != nil {
			return nil, err
		}
	}
	return listenOwnerOnly(path)
}
