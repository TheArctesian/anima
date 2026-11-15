// Command anima-link carries the hub's traffic over an encrypted link
// (docs/one-click-plan.md, Section 4.1). The files of this package wire the
// roles and hold no rules.
package main

import (
	"context"
	"errors"
	"flag"
	"fmt"
	"io"
	"os"
	"os/signal"
	"syscall"
)

const usage = `usage: anima-link hub -state DIR -target 127.0.0.1:PORT [-relay HOST[:PORT] [-replace-relay]] [-name-device] [-stop-with-stdin]
       anima-link device -state DIR [-listen 127.0.0.1:PORT] [-stop-with-stdin]
       anima-link join -state DIR [-hub-name NAME]   (the invite is read from standard input)
       anima-link ctl -state DIR VERB [JSON-ARGUMENTS]`

// stdio is the three streams of the process. Tests give their own.
type stdio struct {
	in  io.Reader
	out io.Writer
	err io.Writer
}

func main() {
	ctx, stop := signal.NotifyContext(context.Background(), syscall.SIGINT, syscall.SIGTERM)
	defer stop()
	os.Exit(run(ctx, os.Args[1:], stdio{os.Stdin, os.Stdout, os.Stderr}))
}

// run does the command that the first argument names and returns the exit
// code: 0 done, 1 failed, 2 wrong use.
func run(ctx context.Context, args []string, streams stdio) int {
	commands := map[string]func(context.Context, []string, stdio) error{
		"hub":    runHub,
		"device": runDevice,
		"join":   runJoin,
		"ctl":    runCtl,
	}
	if len(args) == 0 || commands[args[0]] == nil {
		fmt.Fprintln(streams.err, usage)
		return 2
	}
	err := commands[args[0]](ctx, args[1:], streams)
	switch {
	case err == nil:
		return 0
	case errors.Is(err, errUsage), errors.Is(err, flag.ErrHelp):
		fmt.Fprintln(streams.err, usage)
		return 2
	default:
		fmt.Fprintln(streams.err, "anima-link:", err)
		return 1
	}
}

var errUsage = errors.New("wrong use")

// stateFlags reads the flags that each command has. It returns the flag set
// so that a command can add its own flags before it parses.
func stateFlags(name string) (*flag.FlagSet, *string) {
	flags := flag.NewFlagSet(name, flag.ContinueOnError)
	flags.SetOutput(io.Discard)
	return flags, flags.String("state", "", "the state directory")
}

// stopWhenClosed ends the command when the stream ends. The application
// starts anima-link with a pipe as standard input and never writes to it:
// when the application stops, for any cause, the pipe closes.
func stopWhenClosed(stream io.Reader, stop context.CancelFunc) {
	_, _ = io.Copy(io.Discard, stream)
	stop()
}
