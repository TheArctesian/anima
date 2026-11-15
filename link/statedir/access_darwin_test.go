package statedir_test

import (
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
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
