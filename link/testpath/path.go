// Package testpath supplies private and deliberately exposed filesystem fixtures.
package testpath

import "os"

// MkdirTemp makes a short private path suitable for a local socket on this OS.
func MkdirTemp(prefix string) (string, error) {
	path, err := os.MkdirTemp(root(), prefix)
	if err != nil {
		return "", err
	}
	if err := Chmod(path, 0o700); err != nil {
		os.RemoveAll(path)
		return "", err
	}
	return path, nil
}
