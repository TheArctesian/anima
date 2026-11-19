package statedir_test

import (
	"errors"
	"net"
	"os"
	"path/filepath"
	"testing"

	"github.com/TheArctesian/anima/link/statedir"
)

// privateFolder is a folder that only this user can reach, with a short
// path: a socket path has a limit of about 100 bytes.
func privateFolder(t *testing.T) string {
	t.Helper()
	folder, err := os.MkdirTemp("/tmp", "anima-own-")
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { os.RemoveAll(folder) })
	return folder
}

func TestK8ListenOwnedMakesASocketThatOnlyItsOwnerCanOpen(t *testing.T) {
	path := filepath.Join(privateFolder(t), "link.sock")
	listener, err := statedir.ListenOwned(path)
	if err != nil {
		t.Fatalf("ListenOwned: %v", err)
	}
	defer listener.Close()
	stat, err := os.Lstat(path)
	if err != nil {
		t.Fatal(err)
	}
	if stat.Mode().Type() != os.ModeSocket || stat.Mode().Perm() != 0o600 {
		t.Errorf("the socket has mode %v, want a socket with 0600", stat.Mode())
	}
	conn, err := net.Dial("unix", path)
	if err != nil {
		t.Fatalf("its owner cannot connect: %v", err)
	}
	conn.Close()
}

func TestK8ListenOwnedReplacesTheSocketThatAnEarlierProcessLeft(t *testing.T) {
	path := filepath.Join(privateFolder(t), "link.sock")
	left, err := net.ListenUnix("unix", &net.UnixAddr{Name: path, Net: "unix"})
	if err != nil {
		t.Fatal(err)
	}
	left.SetUnlinkOnClose(false)
	left.Close()
	if err := os.Chmod(path, 0o600); err != nil {
		t.Fatal(err)
	}
	listener, err := statedir.ListenOwned(path)
	if err != nil {
		t.Fatalf("ListenOwned over a socket that was left: %v", err)
	}
	listener.Close()
}

func TestK8ListenOwnedRefusesAFolderThatOtherUsersCanReach(t *testing.T) {
	folder := privateFolder(t)
	if err := os.Chmod(folder, 0o755); err != nil {
		t.Fatal(err)
	}
	listener, err := statedir.ListenOwned(filepath.Join(folder, "link.sock"))
	if err == nil {
		listener.Close()
	}
	if !errors.Is(err, statedir.ErrExposed) {
		t.Errorf("ListenOwned = %v, want ErrExposed", err)
	}
}

func TestK8ListenOwnedDoesNotReplaceAFile(t *testing.T) {
	path := filepath.Join(privateFolder(t), "link.sock")
	if err := os.WriteFile(path, []byte("kept"), 0o600); err != nil {
		t.Fatal(err)
	}
	listener, err := statedir.ListenOwned(path)
	if err == nil {
		listener.Close()
	}
	if !errors.Is(err, statedir.ErrExposed) {
		t.Errorf("ListenOwned = %v, want ErrExposed", err)
	}
	if text, _ := os.ReadFile(path); string(text) != "kept" {
		t.Errorf("the file was changed: %q", text)
	}
}
