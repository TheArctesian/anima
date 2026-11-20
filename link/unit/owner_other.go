//go:build !unix && !windows

package unit

// CheckProgram refuses each program: this system has no unit of the user.
func CheckProgram(program string, _ int) error {
	return refused("no unit of the user on this system for %s", program)
}
