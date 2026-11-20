package statedir_test

import (
	"errors"
	"github.com/TheArctesian/anima/link/testpath"
	"net"
	"os"
	"path/filepath"
	"testing"
	"time"

	"github.com/TheArctesian/anima/link/statedir"
)

// socketIn makes a listening socket that only this user can reach, in a
// new folder with a short path, and returns the folder and the socket.
func socketIn(t *testing.T) (folder, socket string) {
	t.Helper()
	// Under /tmp, not t.TempDir(): a socket's path must be short.
	folder, err := testpath.MkdirTemp("own")
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { os.RemoveAll(folder) })
	socket = filepath.Join(folder, "hub.sock")
	listener, err := net.Listen("unix", socket)
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { listener.Close() })
	if err := testpath.Chmod(socket, 0o600); err != nil {
		t.Fatal(err)
	}
	return folder, socket
}

func TestDialOwnedConnectsToThisUsersSocketInAPrivateFolder(t *testing.T) {
	_, socket := socketIn(t)
	conn, err := statedir.DialOwned(socket, 5*time.Second)
	if err != nil {
		t.Fatalf("DialOwned: %v", err)
	}
	conn.Close()
}

// exposures change a private socket, or the path to it, into what another
// user could put in its place. Each returns the path to dial.
func exposures() map[string]func(t *testing.T, folder, socket string) string {
	return map[string]func(t *testing.T, folder, socket string) string{
		"a file that is not a socket": func(t *testing.T, folder, _ string) string {
			file := filepath.Join(folder, "file")
			if err := os.WriteFile(file, nil, 0o600); err != nil {
				t.Fatal(err)
			}
			return file
		},
		"a symbolic link to the socket": func(t *testing.T, folder, socket string) string {
			link := filepath.Join(folder, "link")
			if err := os.Symlink(socket, link); err != nil {
				t.Fatal(err)
			}
			return link
		},
		"a socket that others can open": func(t *testing.T, _, socket string) string {
			if err := testpath.Chmod(socket, 0o666); err != nil {
				t.Fatal(err)
			}
			return socket
		},
		// The folder is private, but the path goes through a link that any
		// user could turn to a folder of their own after the check.
		"a path through a link in a folder that others can write": func(t *testing.T, folder, _ string) string {
			shared, err := testpath.MkdirTemp("shared")
			if err != nil {
				t.Fatal(err)
			}
			t.Cleanup(func() { os.RemoveAll(shared) })
			if err := testpath.Chmod(shared, 0o777); err != nil {
				t.Fatal(err)
			}
			if err := os.Symlink(folder, filepath.Join(shared, "l")); err != nil {
				t.Fatal(err)
			}
			return filepath.Join(shared, "l", "hub.sock")
		},
	}
}

func TestDialOwnedRefusesWhatAnotherUserCouldPutInItsPlace(t *testing.T) {
	for name, exposed := range exposures() {
		t.Run(name, func(t *testing.T) {
			folder, socket := socketIn(t)
			conn, err := statedir.DialOwned(exposed(t, folder, socket), 5*time.Second)
			if err == nil {
				conn.Close()
			}
			if !errors.Is(err, statedir.ErrExposed) {
				t.Errorf("DialOwned = %v, want ErrExposed", err)
			}
		})
	}
}
