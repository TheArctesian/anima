package main

import (
	"context"
	"log/slog"
	"time"

	"github.com/TheArctesian/anima/link/control"
	"github.com/TheArctesian/anima/link/devices"
	"github.com/TheArctesian/anima/link/hublink"
	"github.com/TheArctesian/anima/link/identity"
	"github.com/TheArctesian/anima/link/statedir"
)

const (
	controlSocket = "control.sock"
	startLimit    = 30 * time.Second
)

func runHub(ctx context.Context, args []string, streams stdio) error {
	flags, statePath := stateFlags("hub")
	target := flags.String("target", "", "the hub's loopback address, host:port")
	stopWithStdin := flags.Bool("stop-with-stdin", false, "stop when standard input closes")
	if err := flags.Parse(args); err != nil || *statePath == "" || *target == "" || flags.NArg() != 0 {
		return errUsage
	}
	log := slog.New(slog.NewTextHandler(streams.err, nil))

	dir, err := statedir.Open(*statePath)
	if err != nil {
		return err
	}
	defer dir.Close()
	link, permitted, err := startHubLink(ctx, dir, *target, log)
	if err != nil {
		return err
	}
	defer link.Close()
	listener, err := dir.Listen(controlSocket)
	if err != nil {
		return err
	}

	ctx, stop := context.WithCancel(ctx)
	defer stop()
	if *stopWithStdin {
		go stopWhenClosed(streams.in, stop)
	}
	log.Info("the hub link is ready", "relay", link.Relay().Name(), "devices", permitted.Count())
	err = control.Serve(ctx, listener, hubVerbs(link, permitted, time.Now))
	log.Info("the hub link stopped")
	return err
}

// startHubLink starts the link from the saved identity and saves the relay
// that a first start chose, so that the address stays the same (LNK-001).
func startHubLink(ctx context.Context, dir *statedir.Dir, target string, log *slog.Logger) (*hublink.Link, *devices.Store, error) {
	hub, err := identity.LoadHub(dir)
	if err != nil {
		return nil, nil, err
	}
	permitted, err := devices.Open(dir)
	if err != nil {
		return nil, nil, err
	}
	ctx, cancel := context.WithTimeout(ctx, startLimit)
	defer cancel()
	link, err := hublink.Start(ctx, hublink.Config{Identity: hub, Target: target, Permits: permitted.Permits, Log: log})
	if err != nil {
		return nil, nil, err
	}
	if hub.Relay.IsZero() {
		hub.Relay = link.Relay()
		if err := hub.Save(dir); err != nil {
			link.Close()
			return nil, nil, err
		}
	}
	return link, permitted, nil
}
