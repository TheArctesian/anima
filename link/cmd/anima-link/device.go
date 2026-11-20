package main

import (
	"context"
	"encoding/json"
	"fmt"
	"log/slog"
	"net"
	"strconv"

	"github.com/TheArctesian/anima/link/control"
	"github.com/TheArctesian/anima/link/devicelink"
	"github.com/TheArctesian/anima/link/identity"
	"github.com/TheArctesian/anima/link/ports"
	"github.com/TheArctesian/anima/link/statedir"
	"github.com/TheArctesian/anima/link/tunnel"
)

// deviceStatus is the answer of the device's "status" verb.
type deviceStatus struct {
	Role string `json:"role"`
	// Local is the loopback address that reaches the hub.
	Local     string           `json:"local"`
	PublicKey tunnel.DeviceKey `json:"publicKey"`
	// Path is "direct", "relayed" or "none" (LNK-006).
	Path      string  `json:"path"`
	LatencyMS float64 `json:"latencyMs"`
	// OpenPorts is the local ports that the hub may reach (HMD-049).
	OpenPorts []uint16 `json:"openPorts"`
}

// openPorts is the answer of the command "ports".
type openPorts struct {
	Open []uint16 `json:"open"`
}

// runPorts shows the list of open ports of the device on a state directory,
// or turns one port on or off (HMD-049). The device link may run: it reads
// the list at each request of the hub, so a change needs no restart.
func runPorts(_ context.Context, args []string, streams stdio) error {
	flags, statePath := stateFlags("ports")
	if err := flags.Parse(args); err != nil || *statePath == "" || (flags.NArg() != 0 && flags.NArg() != 2) {
		return errUsage
	}
	dir, err := statedir.Beside(*statePath)
	if err != nil {
		return err
	}
	if flags.NArg() == 2 {
		port, err := strconv.ParseUint(flags.Arg(1), 10, 16)
		if err != nil || (flags.Arg(0) != "on" && flags.Arg(0) != "off") {
			return errUsage
		}
		if err := ports.Turn(dir, uint16(port), flags.Arg(0) == "on"); err != nil {
			return err
		}
	}
	open, err := ports.List(dir)
	if err != nil {
		return err
	}
	answer, err := json.Marshal(openPorts{Open: append([]uint16{}, open...)})
	if err != nil {
		return err
	}
	_, err = fmt.Fprintln(streams.out, string(answer))
	return err
}

// listenForDaemon makes the socket of the metrics daemon at path; an empty
// path means no socket. The daemon sends its token there, so the socket is
// made before the link starts: a path that other users can reach stops the
// start.
func listenForDaemon(path string) (net.Listener, error) {
	if path == "" {
		return nil, nil
	}
	return statedir.ListenOwned(path)
}

func runDevice(ctx context.Context, args []string, streams stdio) error {
	flags, statePath := stateFlags("device")
	listen := flags.String("listen", "127.0.0.1:0", "the local loopback address, host:port")
	daemonSocket := flags.String("daemon-socket", "", "the path of a Unix socket for the metrics daemon")
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
	forDaemon, err := listenForDaemon(*daemonSocket)
	if err != nil {
		return err
	}
	link, err := devicelink.Start(devicelink.Config{
		Identity: device, Listen: *listen, DaemonSocket: forDaemon,
		Save:     func(renewed identity.Device) error { return renewed.Save(dir) },
		OpenPort: func(port uint16) bool { return ports.IsOpen(dir, port) },
	})
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
	log.Info("the device link is ready", "local", link.LocalAddr().String(), "daemonSocket", *daemonSocket, "key", device.Key.Public())
	err = control.Serve(ctx, listener, map[string]control.Verb{
		"status": func(context.Context, json.RawMessage) (any, error) {
			path := link.Path()
			open, err := ports.List(dir)
			return deviceStatus{
				Role: "device", Local: link.LocalAddr().String(), PublicKey: device.Key.Public(),
				Path: string(path.Kind), LatencyMS: float64(path.Latency.Microseconds()) / 1000,
				OpenPorts: append([]uint16{}, open...),
			}, err
		},
	})
	log.Info("the device link stopped")
	return err
}
