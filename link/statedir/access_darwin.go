//go:build darwin && !ios

package statedir

import (
	"fmt"
	"os/exec"
)

// dropInheritedAccess removes each access-control entry from the directory
// and from what it holds. On macOS a folder above can give such entries to
// everything made in it, and they give access that the mode bits do not show
// (LNK-008). The system has no call for this without C, so it runs chmod.
func dropInheritedAccess(path string) error {
	if err := exec.Command("/bin/chmod", "-R", "-N", path).Run(); err != nil {
		return fmt.Errorf("%w: cannot remove access-control entries from %s", ErrExposed, path)
	}
	return nil
}
