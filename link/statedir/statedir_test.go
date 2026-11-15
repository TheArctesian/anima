package statedir_test

import (
	"errors"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/TheArctesian/anima/link/statedir"
)

type note struct {
	Text string `json:"text"`
}

func open(t *testing.T, path string) *statedir.Dir {
	t.Helper()
	dir, err := statedir.Open(path)
	if err != nil {
		t.Fatalf("Open: %v", err)
	}
	t.Cleanup(func() { dir.Close() })
	return dir
}

func mode(t *testing.T, path string) os.FileMode {
	t.Helper()
	stat, err := os.Stat(path)
	if err != nil {
		t.Fatal(err)
	}
	return stat.Mode().Perm()
}

func TestAFileComesBackAsItWasWritten(t *testing.T) {
	path := filepath.Join(t.TempDir(), "state")
	dir := open(t, path)

	var got note
	if err := dir.Read("note.json", &got); !errors.Is(err, statedir.ErrAbsent) {
		t.Fatalf("before the first write: err = %v, want ErrAbsent", err)
	}
	if err := dir.Write("note.json", note{"first"}); err != nil {
		t.Fatal(err)
	}
	if err := dir.Write("note.json", note{"second"}); err != nil {
		t.Fatal(err)
	}
	if err := dir.Read("note.json", &got); err != nil || got.Text != "second" {
		t.Fatalf("Read = %+v, %v; want the last value written", got, err)
	}
}

func TestLNK008OnlyTheOwnerCanReachTheDirectoryAndItsFiles(t *testing.T) {
	path := filepath.Join(t.TempDir(), "state")
	dir := open(t, path)
	if err := dir.Write("note.json", note{"x"}); err != nil {
		t.Fatal(err)
	}
	if got := mode(t, path); got != 0o700 {
		t.Errorf("directory mode = %o, want 700", got)
	}
	entries, err := os.ReadDir(path)
	if err != nil {
		t.Fatal(err)
	}
	for _, entry := range entries {
		if got := mode(t, filepath.Join(path, entry.Name())); got != 0o600 {
			t.Errorf("%s mode = %o, want 600", entry.Name(), got)
		}
	}
	if len(entries) != 2 {
		t.Errorf("the directory holds %d files, want the lock and the note (no temporary file left)", len(entries))
	}
}

func TestLNK008ADirectoryThatOthersCanReadIsRefused(t *testing.T) {
	path := filepath.Join(t.TempDir(), "state")
	if err := os.Mkdir(path, 0o755); err != nil {
		t.Fatal(err)
	}
	if _, err := statedir.Open(path); !errors.Is(err, statedir.ErrOpen) {
		t.Errorf("Open of a 755 directory: err = %v, want ErrOpen", err)
	}
}

func TestLNK008AFileThatOthersCanReadIsRefused(t *testing.T) {
	path := filepath.Join(t.TempDir(), "state")
	dir := open(t, path)
	if err := dir.Write("note.json", note{"x"}); err != nil {
		t.Fatal(err)
	}
	if err := os.Chmod(filepath.Join(path, "note.json"), 0o644); err != nil {
		t.Fatal(err)
	}
	var got note
	if err := dir.Read("note.json", &got); !errors.Is(err, statedir.ErrOpen) {
		t.Errorf("Read of a 644 file: err = %v, want ErrOpen", err)
	}
}

func TestASecondProcessCannotTakeTheDirectory(t *testing.T) {
	path := filepath.Join(t.TempDir(), "state")
	first := open(t, path)
	if _, err := statedir.Open(path); !errors.Is(err, statedir.ErrInUse) {
		t.Fatalf("second Open: err = %v, want ErrInUse", err)
	}
	first.Close()
	second, err := statedir.Open(path)
	if err != nil {
		t.Fatalf("Open after Close: %v", err)
	}
	second.Close()
}

func TestLNK008ADamagedFileGivesAnErrorWithoutItsContent(t *testing.T) {
	path := filepath.Join(t.TempDir(), "state")
	dir := open(t, path)
	const secret = "privkey:0123456789abcdef"
	if err := os.WriteFile(filepath.Join(path, "note.json"), []byte(`{"text": `+secret), 0o600); err != nil {
		t.Fatal(err)
	}
	var got note
	err := dir.Read("note.json", &got)
	if !errors.Is(err, statedir.ErrDamaged) {
		t.Fatalf("err = %v, want ErrDamaged", err)
	}
	if strings.Contains(err.Error(), "privkey") || strings.Contains(err.Error(), "0123") {
		t.Errorf("the error quotes the file: %q", err)
	}
}
