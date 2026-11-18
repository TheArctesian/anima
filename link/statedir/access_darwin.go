//go:build darwin && !ios

package statedir

import (
	"fmt"
	"io/fs"
	"os/exec"
	"path/filepath"
)

// chmodBatch keeps one chmod command far below the system's limit for the
// length of its arguments.
const chmodBatch = 256

// dropInheritedAccess removes each access-control entry from the directory
// and from what it holds. On macOS a folder above can give such entries to
// everything made in it, and they give access that the mode bits do not show
// (LNK-008). The system has no call for this without C, so it runs chmod.
//
// chmod gets each path by name. Its -R opens each file, and the system
// refuses to open a socket, so -R fails on a directory that holds the socket
// of a running or killed anima-link.
func dropInheritedAccess(path string) error {
	var links, others []string
	err := filepath.WalkDir(path, func(entry string, found fs.DirEntry, err error) error {
		if err != nil {
			return err
		}
		if found.Type()&fs.ModeSymlink != 0 {
			links = append(links, entry)
		} else {
			others = append(others, entry)
		}
		return nil
	})
	if err == nil {
		// -h changes a symbolic link itself, never the file that it names.
		err = chmod(links, "-h", "-N")
	}
	if err == nil {
		err = chmod(others, "-N")
	}
	if err != nil {
		return fmt.Errorf("%w: cannot remove access-control entries from %s", ErrExposed, path)
	}
	return nil
}

func chmod(paths []string, flags ...string) error {
	for len(paths) > 0 {
		batch := paths[:min(chmodBatch, len(paths))]
		paths = paths[len(batch):]
		// Each path is absolute, so chmod reads none of them as a flag.
		if err := exec.Command("/bin/chmod", append(flags[:len(flags):len(flags)], batch...)...).Run(); err != nil {
			return err
		}
	}
	return nil
}
