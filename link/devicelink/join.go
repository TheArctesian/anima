package devicelink

import (
	"bufio"
	"context"
	"encoding/json"
	"errors"
	"fmt"

	"github.com/TheArctesian/anima/link/identity"
	"github.com/TheArctesian/anima/link/invites"
	"github.com/TheArctesian/anima/link/tunnel"
)

// Errors of joining.
var (
	// ErrNoAnswer means that the hub did not answer the invite. The hub
	// gives no answer to an invite that is wrong, used or expired, and a hub
	// that is off gives none: the device cannot tell these apart.
	ErrNoAnswer = errors.New("devicelink: the hub did not answer; the invite is used, expired or wrong, or the hub is off")
	// ErrRefused means that the hub ended the invite but did not permit the
	// device. The invite cannot be used again.
	ErrRefused = errors.New("devicelink: the hub refused this device; ask for a new invite")
)

// Join uses an invite: the device connects with the invite's temporary key
// and gives the hub its own public key (LNK-003). It returns the device's
// identity with the hub address, for the caller to save.
func Join(ctx context.Context, invite invites.Invite, device identity.Device) (identity.Device, error) {
	temporary, err := tunnel.NewDevice(tunnel.DeviceConfig{Key: invite.TemporaryKey(), Hub: invite.Hub})
	if err != nil {
		return identity.Device{}, err
	}
	defer temporary.Close()
	stream, err := temporary.Dial(ctx, tunnel.EnrolPort)
	if err != nil {
		return identity.Device{}, ErrNoAnswer
	}
	defer stream.Close()
	if deadline, ok := ctx.Deadline(); ok {
		_ = stream.SetDeadline(deadline)
	}
	if err := json.NewEncoder(stream).Encode(invites.EnrolRequest{Key: device.Key.Public()}); err != nil {
		return identity.Device{}, ErrNoAnswer
	}
	var reply invites.EnrolReply
	line, err := bufio.NewReader(stream).ReadBytes('\n')
	if err != nil || json.Unmarshal(line, &reply) != nil {
		return identity.Device{}, ErrNoAnswer
	}
	if !reply.OK {
		return identity.Device{}, fmt.Errorf("%w (%s)", ErrRefused, reply.Code)
	}
	device.Hub = invite.Hub
	return device, nil
}
