//go:build unix

package testpath

import (
	"os"
	"testing"
)

func root() string { return "/tmp" }

// Chmod sets the fixture's private or exposed access.
func Chmod(path string, mode os.FileMode) error { return os.Chmod(path, mode) }

// AssertPrivate checks the access the production code left on a fixture.
func AssertPrivate(t testing.TB, path string) {
	t.Helper()
	stat, err := os.Stat(path)
	if err != nil {
		t.Fatal(err)
	}
	want := os.FileMode(0o600)
	if stat.IsDir() {
		want = 0o700
	}
	if stat.Mode().Perm() != want {
		t.Errorf("%s: mode %o, want %o", path, stat.Mode().Perm(), want)
	}
}
