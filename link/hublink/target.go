package hublink

import (
	"errors"
	"net"
	"net/netip"
	"path/filepath"

	"github.com/TheArctesian/anima/link/statedir"
)

// ErrTarget means that the target is neither a loopback address with a port
// nor the path of a socket. The link must not send a device's traffic to
// another computer.
var ErrTarget = errors.New("hublink: the target must be the path of the hub's socket, or a loopback address and a port, for example 127.0.0.1:8443")

// target is where the hub link sends each stream of a device.
type target struct {
	// socket is the path of the hub's Unix socket. Empty means that the
	// target is the loopback address.
	socket   string
	loopback string
}

func parseTarget(text string) (target, error) {
	if filepath.IsAbs(text) {
		return target{socket: filepath.Clean(text)}, nil
	}
	loopback, err := netip.ParseAddrPort(text)
	if err != nil || !loopback.Addr().IsLoopback() || loopback.Port() == 0 {
		return target{}, ErrTarget
	}
	return target{loopback: loopback.String()}, nil
}

// dial connects to the hub. A socket is checked at each connection: the hub
// makes it when it starts, which can be after the link, and makes it again
// at each restart.
func (t target) dial() (net.Conn, error) {
	if t.socket != "" {
		return statedir.DialOwned(t.socket, targetDialLimit)
	}
	return net.DialTimeout("tcp", t.loopback, targetDialLimit)
}
