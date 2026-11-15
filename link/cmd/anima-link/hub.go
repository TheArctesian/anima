package main

import (
	"context"
	"encoding/json"
	"log/slog"

	"github.com/TheArctesian/anima/link/control"
	"github.com/TheArctesian/anima/link/identity"
	"github.com/TheArctesian/anima/link/statedir"
)

const controlSocket = "control.sock"

// hubStatus is the answer of the "status" verb.
type hubStatus struct {
	Role string `json:"role"`
	// Key is the fingerprint of the hub's key, not the key.
	Key string `json:"key"`
}

func runHub(ctx context.Context, args []string, streams stdio) error {
	flags, statePath := stateFlags("hub")
	stopWithStdin := flags.Bool("stop-with-stdin", false, "stop when standard input closes")
	if err := flags.Parse(args); err != nil || *statePath == "" || flags.NArg() != 0 {
		return errUsage
	}
	log := slog.New(slog.NewTextHandler(streams.err, nil))

	dir, err := statedir.Open(*statePath)
	if err != nil {
		return err
	}
	defer dir.Close()
	hub, err := identity.LoadHub(dir)
	if err != nil {
		return err
	}
	listener, err := dir.Listen(controlSocket)
	if err != nil {
		return err
	}

	ctx, stop := context.WithCancel(ctx)
	defer stop()
	if *stopWithStdin {
		go stopWhenClosed(streams.in, stop)
	}
	status := hubStatus{Role: "hub", Key: hub.Key.Public().Fingerprint()}
	log.Info("the hub link is ready", "key", status.Key)
	err = control.Serve(ctx, listener, map[string]control.Verb{
		"status": func(context.Context, json.RawMessage) (any, error) { return status, nil },
	})
	log.Info("the hub link stopped")
	return err
}
