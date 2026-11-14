// Command anima-link carries the hub's traffic over an encrypted link
// (docs/one-click-plan.md, Section 4.1). This file wires the roles and holds
// no rules.
package main

import (
	"fmt"
	"io"
	"os"
)

const usage = "usage: anima-link <hub|device> [flags]"

func main() {
	os.Exit(run(os.Args[1:], os.Stderr))
}

// run starts the role that the first argument names and returns the exit code.
func run(args []string, stderr io.Writer) int {
	if len(args) == 0 {
		fmt.Fprintln(stderr, usage)
		return 2
	}
	switch args[0] {
	case "hub", "device":
		fmt.Fprintf(stderr, "anima-link: the %s role is not built yet\n", args[0])
		return 1
	default:
		fmt.Fprintf(stderr, "anima-link: unknown role %q\n%s\n", args[0], usage)
		return 2
	}
}
