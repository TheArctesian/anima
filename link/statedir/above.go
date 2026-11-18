//go:build !android

package statedir

import (
	"fmt"
	"os"
	"path/filepath"
)

// foldersAbove refuses a directory that another user can replace: a folder
// above it that another user owns, or that others can write with no sticky
// bit.
func foldersAbove(path string) error {
	if err := noForeignLinkAbove(path); err != nil {
		return err
	}
	// The folders above are checked on the real path, so that a symbolic
	// link above cannot hide them.
	resolved, err := filepath.EvalSymlinks(path)
	if err != nil {
		return err
	}
	for above := filepath.Dir(resolved); ; above = filepath.Dir(above) {
		stat, err := os.Lstat(above)
		if err != nil {
			return err
		}
		open := stat.Mode().Perm()&0o022 != 0 && stat.Mode()&os.ModeSticky == 0
		if open || !ownedByRootOrThisUser(stat) {
			return fmt.Errorf("%w: %s", ErrExposed, above)
		}
		if above == filepath.Dir(above) {
			return nil
		}
	}
}

// noForeignLinkAbove checks the path as it is written. A symbolic link in
// it passes only when no other user can change where it leads: the link is
// root's or this user's, and so is its folder, where others cannot replace
// it.
// Without this, another user could turn such a link to a folder of their
// own after the check on the real path.
func noForeignLinkAbove(path string) error {
	for above := filepath.Dir(path); above != filepath.Dir(above); above = filepath.Dir(above) {
		stat, err := os.Lstat(above)
		if err != nil {
			return err
		}
		if stat.Mode()&os.ModeSymlink == 0 {
			continue
		}
		folder, err := os.Lstat(filepath.Dir(above))
		if err != nil {
			return err
		}
		open := folder.Mode().Perm()&0o022 != 0 && folder.Mode()&os.ModeSticky == 0
		if open || !ownedByRootOrThisUser(stat) || !ownedByRootOrThisUser(folder) {
			return fmt.Errorf("%w: %s", ErrExposed, above)
		}
	}
	return nil
}
