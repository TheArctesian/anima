package ports_test

import (
	"errors"
	"path/filepath"
	"runtime"
	"slices"
	"testing"

	"github.com/TheArctesian/anima/link/ports"
	"github.com/TheArctesian/anima/link/statedir"
)

func open(t *testing.T) *statedir.Dir {
	t.Helper()
	dir, err := statedir.Open(filepath.Join(t.TempDir(), "s"))
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { dir.Close() })
	return dir
}

func listed(t *testing.T, dir *statedir.Dir) []uint16 {
	t.Helper()
	list, err := ports.List(dir)
	if err != nil {
		t.Fatal(err)
	}
	return list
}

func TestHMD049TheListIsEmptyUntilAPortIsTurnedOnAndAPortCanBeTurnedOff(t *testing.T) {
	dir := open(t)
	if got := listed(t, dir); len(got) != 0 || ports.IsOpen(dir, 22) {
		t.Fatalf("a new device has the open ports %v", got)
	}
	for _, port := range []uint16{5900, 22, 22} {
		if err := ports.Turn(dir, port, true); err != nil {
			t.Fatal(err)
		}
	}
	if got := listed(t, dir); !slices.Equal(got, []uint16{22, 5900}) {
		t.Fatalf("the list is %v, want 22 and 5900", got)
	}
	if err := ports.Turn(dir, 22, false); err != nil {
		t.Fatal(err)
	}
	if ports.IsOpen(dir, 22) || !ports.IsOpen(dir, 5900) {
		t.Fatalf("after 22 was turned off the list is %v", listed(t, dir))
	}
}

func TestHMD049OnlyThePortsOfTheSessionsCanBeOpen(t *testing.T) {
	dir := open(t)
	for _, port := range []uint16{80, 443, 5432, 0} {
		if err := ports.Turn(dir, port, true); !errors.Is(err, ports.ErrNotAllowed) {
			t.Errorf("port %d: %v, want ErrNotAllowed", port, err)
		}
	}
	if !ports.Allowed(3389, "windows") || ports.Allowed(3389, "darwin") || ports.Allowed(3389, "linux") {
		t.Fatal("port 3389 must be allowed on Windows only")
	}
	if err := ports.Turn(dir, 3389, true); (err == nil) != (runtime.GOOS == "windows") {
		t.Fatalf("port 3389 on %s: %v", runtime.GOOS, err)
	}
}

// The file is the person's, but a port that the system does not allow is
// not open, also when a hand wrote it in the file.
func TestHMD049APortThatAHandWroteInTheFileIsNotOpen(t *testing.T) {
	dir := open(t)
	if err := dir.Write("open-ports", []uint16{22, 80, 8080, 22}); err != nil {
		t.Fatal(err)
	}
	if got := listed(t, dir); !slices.Equal(got, []uint16{22}) {
		t.Fatalf("the list is %v, want only 22", got)
	}
	if ports.IsOpen(dir, 80) {
		t.Fatal("port 80 is open")
	}
}

// A second process changes the list while the device link holds the
// directory: the link reads the change with no restart.
func TestHMD049AChangeBesideTheRunningLinkAppliesAtOnce(t *testing.T) {
	path := filepath.Join(t.TempDir(), "s")
	held, err := statedir.Open(path)
	if err != nil {
		t.Fatal(err)
	}
	defer held.Close()
	beside, err := statedir.Beside(path)
	if err != nil {
		t.Fatalf("Beside: %v", err)
	}
	defer beside.Close()
	if err := ports.Turn(beside, 5900, true); err != nil {
		t.Fatal(err)
	}
	if !ports.IsOpen(held, 5900) {
		t.Fatal("the link does not see the port that the second process turned on")
	}
}
