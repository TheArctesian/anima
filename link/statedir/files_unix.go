//go:build !windows

package statedir

import (
	"io/fs"
	"os"
	"syscall"
)

func makeDirectory(path string) error { return os.MkdirAll(path, 0o700) }
func lockFile(file *os.File) error {
	return syscall.Flock(int(file.Fd()), syscall.LOCK_EX|syscall.LOCK_NB)
}
func readFile(path string) (*os.File, error) {
	return os.OpenFile(path, os.O_RDONLY|syscall.O_NOFOLLOW|syscall.O_NONBLOCK, 0)
}
func syncDirectory(file *os.File) error { return file.Sync() }

func ownedByRootOrThisUser(stat fs.FileInfo) bool {
	owner, known := stat.Sys().(*syscall.Stat_t)
	return !known || owner.Uid == 0 || int(owner.Uid) == os.Getuid()
}

// ownerOnly reports whether only this user can reach the file.
func ownerOnly(stat fs.FileInfo, _ string) bool {
	owner, known := stat.Sys().(*syscall.Stat_t)
	return stat.Mode().Perm()&0o077 == 0 && (!known || int(owner.Uid) == os.Getuid())
}
