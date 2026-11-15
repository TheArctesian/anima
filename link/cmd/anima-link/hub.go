package main

import (
	"context"
	"errors"
	"log/slog"
	"maps"
	"time"

	"github.com/TheArctesian/anima/link/control"
	"github.com/TheArctesian/anima/link/devices"
	"github.com/TheArctesian/anima/link/hublink"
	"github.com/TheArctesian/anima/link/identity"
	"github.com/TheArctesian/anima/link/invites"
	"github.com/TheArctesian/anima/link/statedir"
	"github.com/TheArctesian/anima/link/tunnel"
)

const (
	controlSocket = "control.sock"
	startLimit    = 30 * time.Second
)

func runHub(ctx context.Context, args []string, streams stdio) error {
	flags, statePath := stateFlags("hub")
	target := flags.String("target", "", "the hub's loopback address, host:port")
	relay := flags.String("relay", "", "the hub's own relay, host or host:port; at the first start only")
	replaceRelay := flags.Bool("replace-relay", false, "with -relay: take this relay although the hub has one; its devices cannot follow")
	nameDevice := flags.Bool("name-device", false, "start each connection to the hub with a PROXY header that holds the device key")
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
	if err := chooseRelay(dir, *relay, *replaceRelay); err != nil {
		return err
	}
	link, lists, err := startHubLink(ctx, dir, hubOptions{target: *target, nameDevice: *nameDevice, log: log})
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
	verbs := hubVerbs(link, lists.devices)
	maps.Copy(verbs, inviteVerbs(link, lists.invites, time.Now))
	maps.Copy(verbs, relayVerbs(link, dir, stop))
	err = control.Serve(ctx, listener, verbs)
	log.Info("the hub link stopped")
	return err
}

var errRelayIsSet = errors.New("this hub has a relay already; change it with the verbs relay.announce and relay.switch, so that its devices can follow. " +
	"If the hub cannot start on its relay, start it with -relay and -replace-relay")

// chooseRelay saves the operator's own relay before the first start
// (LNK-007). A hub that has a relay keeps it: a different relay is a
// different address. With replace, the operator takes another relay
// anyway: the way back for a hub whose saved relay does not work, for
// example after a relay.switch to a mistyped name.
func chooseRelay(dir *statedir.Dir, hostPort string, replace bool) error {
	if hostPort == "" {
		return nil
	}
	relay, err := tunnel.OwnRelay(hostPort)
	if err != nil {
		return err
	}
	hub, err := identity.LoadHub(dir)
	if err != nil {
		return err
	}
	if !hub.Relay.IsZero() && !replace {
		if hub.Relay.Name() == relay.Name() {
			return nil
		}
		return errRelayIsSet
	}
	hub.Relay, hub.NextRelay = relay, tunnel.Relay{}
	return hub.Save(dir)
}

// startHubLink starts the link from the saved identity and saves the relay
// that a first start chose, so that the address stays the same (LNK-001).
// hubOptions is what the command line gives a hub link.
type hubOptions struct {
	target     string
	nameDevice bool
	log        *slog.Logger
}

func startHubLink(ctx context.Context, dir *statedir.Dir, options hubOptions) (*hublink.Link, hubLists, error) {
	hub, err := identity.LoadHub(dir)
	if err != nil {
		return nil, hubLists{}, err
	}
	permitted, err := devices.Open(dir, time.Now)
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
		Identity: hub, Target: options.target, NameDevice: options.nameDevice,
		Devices: permitted, Invites: open, Log: options.log,
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
