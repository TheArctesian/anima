package statedir

import (
	"errors"
	"os"
	"path/filepath"
	"testing"
	"time"

	"golang.org/x/sys/windows"
)

func TestWindowsStateLifecycle(t *testing.T) {
	path := filepath.Join(t.TempDir(), "state")
	dir, err := Open(path)
	if err != nil {
		t.Fatal(err)
	}
	defer dir.Close()
	if _, err := Open(path); !errors.Is(err, ErrInUse) {
		t.Fatalf("second writer: %v", err)
	}
	if err := dir.Write("example", map[string]string{"value": "first"}); err != nil {
		t.Fatal(err)
	}
	if err := dir.Write("example", map[string]string{"value": "second"}); err != nil {
		t.Fatal(err)
	}
	var got map[string]string
	if err := dir.Read("example", &got); err != nil {
		t.Fatal(err)
	}
	if got["value"] != "second" {
		t.Fatalf("read: %v", got)
	}
}

func TestWindowsExposedStateIsRefused(t *testing.T) {
	path := filepath.Join(t.TempDir(), "state")
	dir, err := Open(path)
	if err != nil {
		t.Fatal(err)
	}
	dir.Close()
	sid, err := currentSID()
	if err != nil {
		t.Fatal(err)
	}
	sd, err := windows.SecurityDescriptorFromString("D:P(A;;FA;;;" + sid.String() + ")(A;;FR;;;WD)")
	if err != nil {
		t.Fatal(err)
	}
	acl, _, err := sd.DACL()
	if err != nil {
		t.Fatal(err)
	}
	if err := windows.SetNamedSecurityInfo(path, windows.SE_FILE_OBJECT, windows.DACL_SECURITY_INFORMATION|windows.PROTECTED_DACL_SECURITY_INFORMATION, nil, nil, acl, nil); err != nil {
		t.Fatal(err)
	}
	if _, err := Open(path); !errors.Is(err, ErrExposed) {
		t.Fatalf("exposed directory: %v", err)
	}
}

func TestWindowsReparseReadIsRefused(t *testing.T) {
	root := t.TempDir()
	target := filepath.Join(root, "target")
	if err := os.WriteFile(target, []byte("private"), 0600); err != nil {
		t.Fatal(err)
	}
	link := filepath.Join(root, "link")
	if err := os.Symlink(target, link); err != nil {
		t.Skip("symlink privilege unavailable")
	}
	if file, err := readFile(link); err == nil {
		file.Close()
		t.Fatal("followed a reparse point")
	}
}

func TestWindowsControlSocket(t *testing.T) {
	root, err := os.MkdirTemp("", "sock")
	if err != nil {
		t.Fatal(err)
	}
	defer os.RemoveAll(root)
	dir, err := Open(filepath.Join(root, "state"))
	if err != nil {
		t.Fatal(err)
	}
	defer dir.Close()
	listener, err := dir.Listen("control.sock")
	if err != nil {
		t.Fatal(err)
	}
	defer listener.Close()
	conn, err := DialOwned(filepath.Join(root, "state", "control.sock"), time.Second)
	if err != nil {
		t.Fatal(err)
	}
	conn.Close()
}
