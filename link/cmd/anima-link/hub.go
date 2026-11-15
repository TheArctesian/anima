package main

import (
	"context"
	"log/slog"
	"maps"
	"time"

	"github.com/TheArctesian/anima/link/control"
	"github.com/TheArctesian/anima/link/devices"
	"github.com/TheArctesian/anima/link/hublink"
	"github.com/TheArctesian/anima/link/identity"
	"github.com/TheArctesian/anima/link/invites"
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
	link, lists, err := startHubLink(ctx, dir, *target, log)
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
	log.Info("the hub link is ready", "relay", link.Relay().Name(), "devices", lists.devices.Count())
	verbs := hubVerbs(link, lists.devices, time.Now)
	maps.Copy(verbs, inviteVerbs(link, lists.invites, time.Now))
	err = control.Serve(ctx, listener, verbs)
	log.Info("the hub link stopped")
	return err
}

// startHubLink starts the link from the saved identity and saves the relay
// that a first start chose, so that the address stays the same (LNK-001).
func startHubLink(ctx context.Context, dir *statedir.Dir, target string, log *slog.Logger) (*hublink.Link, hubLists, error) {
	hub, err := identity.LoadHub(dir)
	if err != nil {
		return nil, hubLists{}, err
	}
	permitted, err := devices.Open(dir)
	if err != nil {
		return nil, hubLists{}, err
	}
	open, err := invites.Open(dir)
	if err != nil {
		return nil, hubLists{}, err
	}
	ctx, cancel := context.WithTimeout(ctx, startLimit)
	defer cancel()
	link, err := hublink.Start(ctx, hublink.Config{
		Identity: hub, Target: target, Permits: permitted.Permits, Log: log,
		Enrolment: &hublink.Enrolment{Invites: open, Devices: permitted, Now: time.Now},
	})
	if err != nil {
		return nil, hubLists{}, err
	}
	if hub.Relay.IsZero() {
		hub.Relay = link.Relay()
		if err := hub.Save(dir); err != nil {
			link.Close()
			return nil, hubLists{}, err
		}
	}
	return link, hubLists{permitted, open}, nil
}

// hubLists is what a hub link keeps in its state directory beside its identity.
type hubLists struct {
	devices *devices.Store
	invites *invites.Store
}
