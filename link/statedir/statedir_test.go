package statedir_test

import (
	"context"
	"errors"
	"github.com/TheArctesian/anima/link/testpath"
	"net"
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
	testpath.AssertPrivate(t, path)
	entries, err := os.ReadDir(path)
	if err != nil {
		t.Fatal(err)
	}
	for _, entry := range entries {
		testpath.AssertPrivate(t, filepath.Join(path, entry.Name()))
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
	if err := testpath.Chmod(path, 0o755); err != nil {
		t.Fatal(err)
	}
	if _, err := statedir.Open(path); !errors.Is(err, statedir.ErrExposed) {
		t.Errorf("Open of a 755 directory: err = %v, want ErrExposed", err)
	}
}

func TestLNK008AFileThatOthersCanReadIsRefused(t *testing.T) {
	path := filepath.Join(t.TempDir(), "state")
	dir := open(t, path)
	if err := dir.Write("note.json", note{"x"}); err != nil {
		t.Fatal(err)
	}
	if err := testpath.Chmod(filepath.Join(path, "note.json"), 0o644); err != nil {
		t.Fatal(err)
	}
	var got note
	if err := dir.Read("note.json", &got); !errors.Is(err, statedir.ErrExposed) {
		t.Errorf("Read of a 644 file: err = %v, want ErrExposed", err)
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

func shortDir(t *testing.T) string {
	t.Helper()
	path, err := testpath.MkdirTemp("anima-state-")
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { os.RemoveAll(path) })
	return path
}

func TestLNK008OnlyTheOwnerCanConnectToASocketOfTheDirectory(t *testing.T) {
	path := shortDir(t)
	dir := open(t, path)
	listener, err := dir.Listen("c.sock")
	if err != nil {
		t.Fatal(err)
	}
	defer listener.Close()
	testpath.AssertPrivate(t, filepath.Join(path, "c.sock"))
	conn, err := statedir.Dial(context.Background(), path, "c.sock")
	if err != nil {
		t.Fatalf("the owner cannot connect: %v", err)
	}
	conn.Close()
}

func TestASocketLeftByAnEarlierProcessIsReplaced(t *testing.T) {
	path := shortDir(t)
	first := open(t, path)
	listener, err := first.Listen("c.sock")
	if err != nil {
		t.Fatal(err)
	}
	// A killed process leaves its socket file. Keep the file when the listener closes.
	listener.(*net.UnixListener).SetUnlinkOnClose(false)
	listener.Close()
	first.Close()

	second := open(t, path)
	again, err := second.Listen("c.sock")
	if err != nil {
		t.Fatalf("Listen over a stale socket: %v", err)
	}
	again.Close()
}

func TestAStateDirectoryWithATooLongPathCannotHaveASocket(t *testing.T) {
	dir := open(t, filepath.Join(shortDir(t), strings.Repeat("d", 100)))
	if _, err := dir.Listen("c.sock"); !errors.Is(err, statedir.ErrLongPath) {
		t.Errorf("err = %v, want ErrLongPath", err)
	}
}

func TestLNK008ASymbolicLinkIsNotAStateDirectoryOrAStateFile(t *testing.T) {
	base := t.TempDir()
	target := filepath.Join(base, "real")
	dir := open(t, target)
	if err := dir.Write("note.json", note{"x"}); err != nil {
		t.Fatal(err)
	}
	if err := os.Symlink(filepath.Join(target, "note.json"), filepath.Join(target, "link.json")); err != nil {
		t.Fatal(err)
	}
	var got note
	if err := dir.Read("link.json", &got); !errors.Is(err, statedir.ErrExposed) {
		t.Errorf("Read through a symbolic link: err = %v, want ErrExposed", err)
	}
	dir.Close()

	linked := filepath.Join(base, "linked")
	if err := os.Symlink(target, linked); err != nil {
		t.Fatal(err)
	}
	if _, err := statedir.Open(linked); !errors.Is(err, statedir.ErrExposed) {
		t.Errorf("Open of a symbolic link: err = %v, want ErrExposed", err)
	}
}

func TestLNK008ADirectoryThatAnotherUserCanReplaceIsRefused(t *testing.T) {
	parent := filepath.Join(t.TempDir(), "shared")
	if err := os.Mkdir(parent, 0o700); err != nil {
		t.Fatal(err)
	}
	if err := testpath.Chmod(parent, 0o777); err != nil {
		t.Fatal(err)
	}
	if _, err := statedir.Open(filepath.Join(parent, "state")); !errors.Is(err, statedir.ErrExposed) {
		t.Errorf("Open under a folder that all can write: err = %v, want ErrExposed", err)
	}
}

func TestLNK008ALinkAboveCannotHideAFolderThatOthersCanWrite(t *testing.T) {
	base := t.TempDir()
	shared := filepath.Join(base, "shared")
	if err := os.MkdirAll(filepath.Join(shared, "inner"), 0o700); err != nil {
		t.Fatal(err)
	}
	if err := testpath.Chmod(shared, 0o777); err != nil {
		t.Fatal(err)
	}
	link := filepath.Join(base, "link")
	if err := os.Symlink(filepath.Join(shared, "inner"), link); err != nil {
		t.Fatal(err)
	}
	if _, err := statedir.Open(filepath.Join(link, "state")); !errors.Is(err, statedir.ErrExposed) {
		t.Errorf("Open through a link into a folder that all can write: err = %v, want ErrExposed", err)
	}
}
