package main

import (
	"context"
	"encoding/json"
	"fmt"
	"time"

	"github.com/TheArctesian/anima/link/control"
	"github.com/TheArctesian/anima/link/statedir"
)

// runCtl sends one verb to the anima-link that runs on a state directory
// and prints the JSON answer.
func runCtl(ctx context.Context, args []string, streams stdio) error {
	flags, statePath := stateFlags("ctl")
	if err := flags.Parse(args); err != nil || *statePath == "" || flags.NArg() < 1 || flags.NArg() > 2 {
		return errUsage
	}
	arguments := json.RawMessage("null")
	if flags.NArg() == 2 {
		arguments = json.RawMessage(flags.Arg(1))
		if !json.Valid(arguments) {
			return errUsage
		}
	}
	ctx, cancel := context.WithTimeout(ctx, 30*time.Second)
	defer cancel()
	conn, err := statedir.Dial(ctx, *statePath, controlSocket)
	if err != nil {
		return fmt.Errorf("no anima-link runs on this state directory: %w", err)
	}
	defer conn.Close()
	if deadline, ok := ctx.Deadline(); ok {
		_ = conn.SetDeadline(deadline)
	}
	var result json.RawMessage
	if err := control.Call(conn, flags.Arg(0), arguments, &result); err != nil {
		return err
	}
	_, err = fmt.Fprintln(streams.out, string(result))
	return err
}
