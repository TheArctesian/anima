package statedir_test

import (
	"errors"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"

	"github.com/TheArctesian/anima/link/statedir"
)

func TestLNK008AnInheritedAccessEntryDoesNotReachTheStateFiles(t *testing.T) {
	parent := filepath.Join(t.TempDir(), "parent")
	if err := os.Mkdir(parent, 0o700); err != nil {
		t.Fatal(err)
	}
	grant := "everyone allow read,list,search,file_inherit,directory_inherit"
	if out, err := exec.Command("/bin/chmod", "+a", grant, parent).CombinedOutput(); err != nil {
		t.Skipf("this file system has no access-control entries: %v %s", err, out)
	}
	path := filepath.Join(parent, "state")
	dir := open(t, path)
	if err := dir.Write("note.json", note{"x"}); err != nil {
		t.Fatal(err)
	}

	// The directory itself (-d), then what it holds.
	listing, err := exec.Command("/bin/ls", "-lde", path).CombinedOutput()
	if err != nil {
		t.Fatal(err)
	}
	inside, err := exec.Command("/bin/ls", "-le", path).CombinedOutput()
	if err != nil {
		t.Fatal(err)
	}
	listing = append(listing, inside...)
	if strings.Contains(string(listing), "allow") {
		t.Errorf("an access-control entry is on the state directory or a file in it:\n%s", listing)
	}
}

func TestARefusedDirectoryKeepsItsAccessEntries(t *testing.T) {
	path := filepath.Join(t.TempDir(), "not-a-state-directory")
	if err := os.Mkdir(path, 0o755); err != nil {
		t.Fatal(err)
	}
	if out, err := exec.Command("/bin/chmod", "+a", "everyone allow read", path).CombinedOutput(); err != nil {
		t.Skipf("this file system has no access-control entries: %v %s", err, out)
	}
	if _, err := statedir.Open(path); !errors.Is(err, statedir.ErrExposed) {
		t.Fatalf("Open of a 755 directory: err = %v, want ErrExposed", err)
	}
	listing, err := exec.Command("/bin/ls", "-lde", path).CombinedOutput()
	if err != nil {
		t.Fatal(err)
	}
	if !strings.Contains(string(listing), "allow") {
		t.Errorf("Open changed a directory that it refused:\n%s", listing)
	}
}
