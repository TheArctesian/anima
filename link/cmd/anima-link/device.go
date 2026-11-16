package main

import (
	"context"
	"encoding/json"
	"log/slog"

	"github.com/TheArctesian/anima/link/control"
	"github.com/TheArctesian/anima/link/devicelink"
	"github.com/TheArctesian/anima/link/identity"
	"github.com/TheArctesian/anima/link/statedir"
)

// deviceStatus is the answer of the device's "status" verb.
type deviceStatus struct {
	Role string `json:"role"`
	// Local is the loopback address that reaches the hub.
	Local string `json:"local"`
	// Path is "direct", "relayed" or "none" (LNK-006).
	Path      string  `json:"path"`
	LatencyMS float64 `json:"latencyMs"`
}

func runDevice(ctx context.Context, args []string, streams stdio) error {
	flags, statePath := stateFlags("device")
	listen := flags.String("listen", "127.0.0.1:0", "the local loopback address, host:port")
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
	device, err := identity.LoadDevice(dir)
	if err != nil {
		return err
	}
	link, err := devicelink.Start(devicelink.Config{Identity: device, Listen: *listen, Save: func(renewed identity.Device) error {
		return renewed.Save(dir)
	}})
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
	log.Info("the device link is ready", "local", link.LocalAddr().String(), "key", device.Key.Public())
	err = control.Serve(ctx, listener, map[string]control.Verb{
		"status": func(context.Context, json.RawMessage) (any, error) {
			path := link.Path()
			return deviceStatus{
				Role: "device", Local: link.LocalAddr().String(),
				Path: string(path.Kind), LatencyMS: float64(path.Latency.Microseconds()) / 1000,
			}, nil
		},
	})
	log.Info("the device link stopped")
	return err
}
