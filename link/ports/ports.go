// Package ports is the list of open ports of a device (HMD-049): the local
// ports of this computer that its hub may reach through the link (HMD-048).
// The list is a file in the state directory of the device. The hub cannot
// change it; only the person on the computer can. It is empty until the
// person turns a port on.
package ports

import (
	"errors"
	"runtime"
	"slices"

	"github.com/TheArctesian/anima/link/statedir"
)

// ErrNotAllowed means that the port cannot be in the list on this system.
var ErrNotAllowed = errors.New("ports: only 22 (SSH), 5900 (screen sharing) and, on Windows, 3389 (remote desktop) can be open")

// fileName is the file of the list: a JSON array of port numbers.
const fileName = "open-ports"

// Allowed reports whether a port can be in the list on the system goos (a
// value of runtime.GOOS).
func Allowed(port uint16, goos string) bool {
	switch port {
	case 22, 5900:
		return true
	case 3389:
		return goos == "windows"
	default:
		return false
	}
}

// List reads the open ports, in order. An absent file is an empty list. A
// port that this system does not allow is left out, also when the file
// names it.
func List(dir *statedir.Dir) ([]uint16, error) {
	var named []uint16
	if err := dir.Read(fileName, &named); err != nil {
		if errors.Is(err, statedir.ErrAbsent) {
			return nil, nil
		}
		return nil, err
	}
	open := make([]uint16, 0, len(named))
	for _, port := range named {
		if Allowed(port, runtime.GOOS) && !slices.Contains(open, port) {
			open = append(open, port)
		}
	}
	slices.Sort(open)
	return open, nil
}

// IsOpen reports whether a port is in the list now. It reads the file at
// each call, so that a change applies with no restart. A list that cannot
// be read has no open port.
func IsOpen(dir *statedir.Dir, port uint16) bool {
	open, err := List(dir)
	return err == nil && slices.Contains(open, port)
}

// Turn puts a port in the list or takes it out.
func Turn(dir *statedir.Dir, port uint16, on bool) error {
	if !Allowed(port, runtime.GOOS) {
		return ErrNotAllowed
	}
	open, err := List(dir)
	if err != nil {
		return err
	}
	open = slices.DeleteFunc(append([]uint16{}, open...), func(listed uint16) bool { return listed == port })
	if on {
		open = append(open, port)
		slices.Sort(open)
	}
	return dir.Write(fileName, open)
}
