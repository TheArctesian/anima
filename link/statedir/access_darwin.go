//go:build darwin && !ios

package statedir

import (
	"fmt"
	"io/fs"
	"os/exec"
	"path/filepath"
)

// dropInheritedAccess removes each access-control entry from the directory
// and from what it holds. On macOS a folder above can give such entries to
// everything made in it, and they give access that the mode bits do not show
// (LNK-008). The system has no call for this without C, so it runs chmod.
//
// A socket is left out: chmod cannot open one, and fails for the whole
// directory when a process that was killed left its socket there. No other
// user reaches a socket of the directory, because the directory itself has
// no entry after this call, and Listen replaces the socket.
func dropInheritedAccess(path string) error {
	exposed := fmt.Errorf("%w: cannot remove access-control entries from %s", ErrExposed, path)
	arguments := []string{"-N"}
	err := filepath.WalkDir(path, func(entry string, found fs.DirEntry, err error) error {
		if err == nil && found.Type()&fs.ModeSocket == 0 {
			arguments = append(arguments, entry)
		}
		return err
	})
	if err != nil || exec.Command("/bin/chmod", arguments...).Run() != nil {
		return exposed
	}
	return nil
}
