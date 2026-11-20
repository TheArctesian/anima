package main

import (
	"context"
	"errors"
	"net"
	"runtime"
	"sync"
	"time"

	"github.com/TheArctesian/anima/link/devicelink"
	"github.com/TheArctesian/anima/link/identity"
	"github.com/TheArctesian/anima/link/invites"
	"github.com/TheArctesian/anima/link/statedir"
)

// joinLimit is how long a join waits for the hub, as for anima-link join.
const joinLimit = 30 * time.Second

var errBadInvite = errors.New("phonelink: this is not an invite")

// joined is the answer of a join: the fingerprint of the device's key, which the operator can find on the hub.
type joined struct {
	Fingerprint string `json:"fingerprint"`
}

// started is the answer of a start: the loopback address that reaches the hub.
type started struct {
	Local string `json:"local"`
}

// linkStatus is the answer of a status. Path is "direct", "relayed" or "none" (LNK-006).
type linkStatus struct {
	Running   bool    `json:"running"`
	Local     string  `json:"local,omitempty"`
	Path      string  `json:"path,omitempty"`
	LatencyMS float64 `json:"latencyMs,omitempty"`
	DeviceKey string  `json:"deviceKey,omitempty"`
	PublicKey string  `json:"publicKey,omitempty"`
}

// failure is the answer of a call that failed: a stable code, never a path, a key or the invite.
type failure struct {
	Error string `json:"error"`
}

// The one link of the app, and the directory that it holds while it runs.
var (
	guard     sync.Mutex
	running   *devicelink.Link
	held      *statedir.Dir
	deviceKey string
	publicKey string
)

// join uses the invite in line with the identity in the directory state, and saves the joined identity.
func join(state, line string) (joined, error) {
	invite, err := invites.Parse(line)
	if err != nil {
		return joined{}, errBadInvite
	}
	guard.Lock()
	defer guard.Unlock()
	dir, err := statedir.Open(state)
	if err != nil {
		return joined{}, err
	}
	defer dir.Close()
	device, err := identity.LoadDevice(dir)
	if err != nil {
		return joined{}, err
	}
	ctx, cancel := context.WithTimeout(context.Background(), joinLimit)
	defer cancel()
	done, err := devicelink.Join(ctx, invite, device)
	if err != nil {
		return joined{}, err
	}
	if err := done.Save(dir); err != nil {
		return joined{}, err
	}
	return joined{Fingerprint: done.Key.Public().Fingerprint()}, nil
}

// start starts the link of the device in the directory state on listen; a link that runs already stays.
func start(state, listen string) (started, error) {
	guard.Lock()
	defer guard.Unlock()
	if running != nil {
		return started{Local: running.LocalAddr().String()}, nil
	}
	dir, err := statedir.Open(state)
	if err != nil {
		return started{}, err
	}
	device, err := identity.LoadDevice(dir)
	if err != nil {
		_ = dir.Close()
		return started{}, err
	}
	key := device.Key.Public()
	socket := metricsSocket(dir, key.Fingerprint())
	link, err := devicelink.Start(devicelink.Config{Identity: device, Listen: listen, DaemonSocket: socket, Save: func(renewed identity.Device) error {
		return renewed.Save(dir)
	}})
	if err != nil {
		_ = dir.Close()
		return started{}, err
	}
	running, held = link, dir
	deviceKey = key.Fingerprint()
	encoded, _ := key.MarshalText() // A populated public key always has a text form.
	publicKey = string(encoded)
	return started{Local: link.LocalAddr().String()}, nil
}

func metricsSocket(dir *statedir.Dir, fingerprint string) net.Listener {
	if runtime.GOOS != "android" {
		return nil
	}
	// Keep the UI link available even when Android cannot open the reporting socket.
	socket, _ := dir.Listen("metrics-" + fingerprint + ".sock")
	return socket
}

// status reads the link that runs, if one does.
func status() linkStatus {
	guard.Lock()
	defer guard.Unlock()
	if running == nil {
		return linkStatus{}
	}
	path := running.Path()
	return linkStatus{
		Running: true, Local: running.LocalAddr().String(),
		Path: string(path.Kind), LatencyMS: float64(path.Latency.Microseconds()) / 1000,
		DeviceKey: deviceKey, PublicKey: publicKey,
	}
}

// stop stops the link that runs and lets its directory go.
func stop() {
	guard.Lock()
	defer guard.Unlock()
	if running == nil {
		return
	}
	_ = running.Close()
	_ = held.Close()
	running, held = nil, nil
	deviceKey = ""
	publicKey = ""
}

// code is the stable code of err that the shell gives the client, as the desktop link's codes.
func code(err error) string {
	switch {
	case errors.Is(err, errBadInvite):
		return "bad-invite"
	case errors.Is(err, devicelink.ErrNoAnswer):
		return "no-answer"
	case errors.Is(err, devicelink.ErrRefused):
		return "refused"
	case errors.Is(err, devicelink.ErrNotEnrolled):
		return "not-joined"
	case errors.Is(err, statedir.ErrInUse), errors.Is(err, statedir.ErrExposed), errors.Is(err, statedir.ErrDamaged),
		errors.Is(err, statedir.ErrLongPath):
		return "folder"
	default:
		return "failed"
	}
}
