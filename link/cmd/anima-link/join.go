package main

import (
	"bufio"
	"context"
	"fmt"
	"time"

	"github.com/TheArctesian/anima/link/devicelink"
	"github.com/TheArctesian/anima/link/identity"
	"github.com/TheArctesian/anima/link/invites"
	"github.com/TheArctesian/anima/link/statedir"
)

const joinLimit = 30 * time.Second

// runJoin uses an invite. The invite comes on standard input, not as an
// argument: other users of the computer can read the arguments of a process
// (LNK-008).
func runJoin(ctx context.Context, args []string, streams stdio) error {
	flags, statePath := stateFlags("join")
	if err := flags.Parse(args); err != nil || *statePath == "" || flags.NArg() != 0 {
		return errUsage
	}
	line, _ := bufio.NewReader(streams.in).ReadString('\n')
	invite, err := invites.Parse(line)
	if err != nil {
		return err
	}
	dir, err := statedir.Open(*statePath)
	if err != nil {
		return err
	}
	defer dir.Close()
	device, err := identity.LoadDevice(dir)
	if err != nil {
		return err
	}
	ctx, cancel := context.WithTimeout(ctx, joinLimit)
	defer cancel()
	joined, err := devicelink.Join(ctx, invite, device)
	if err != nil {
		return err
	}
	if err := joined.Save(dir); err != nil {
		return err
	}
	_, err = fmt.Fprintln(streams.out, "This device has joined the hub. Start the link with: anima-link device -state", *statePath)
	return err
}
