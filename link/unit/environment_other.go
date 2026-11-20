//go:build !windows

package unit

import "os"

// Only system programs are searched; no caller search path is inherited.
const searchPath = "PATH=/usr/sbin:/usr/bin:/sbin:/bin"

func prepareDirectory(path string) error { return os.MkdirAll(path, 0o755) }

func commandEnvironment() ([]string, string) { return []string{searchPath}, "/" }

func unitBytes(content string) []byte { return []byte(content) }
