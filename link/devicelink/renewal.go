package devicelink

import (
	"context"
	"time"

	"github.com/TheArctesian/anima/link/devices"
	"github.com/TheArctesian/anima/link/invites"
	"github.com/TheArctesian/anima/link/tunnel"
)

const renewStepLimit = 5 * time.Second

// renewIfDue replaces the device's key in the last part of its life, with
// no action by the user (LNK-005). The steps keep one working key at each
// moment, also when the device stops between them:
//
//  1. make the next key and save it beside the current key;
//  2. give its public half to the hub, over the link, with the current key;
//  3. connect with the next key and confirm it. The hub then makes it the
//     device's key and drops the old key;
//  4. save the next key as the current key, and use it.
//
// After a stop, the device starts again at step 2 with the same next key.
// When the hub has already dropped the old key, step 2 gets no answer and
// step 3 still works.
//
// Known limit: the sessions that are open when step 3 completes end. They
// use the old key's tunnel, and the hub disconnects the old key. This
// happens once in about 150 days for each device.
func (l *Link) renewIfDue(ctx context.Context) {
	current := l.identity()
	if l.save == nil || current.Expires.IsZero() {
		return
	}
	if current.NextKey.IsZero() {
		if l.now().Before(current.Expires.Add(-devices.RenewWindow)) {
			return
		}
		current.NextKey = tunnel.NewPrivateKey()
		if l.save(current) != nil {
			return
		}
		l.setIdentity(current)
	}
	next, err := tunnel.NewDevice(tunnel.DeviceConfig{Key: current.NextKey, Hub: current.Hub, DebugLogf: l.debugLogf})
	if err != nil {
		return
	}
	_, _ = step(ctx, l.tunnel(), current.NextKey.Public())
	confirmed, err := step(ctx, next, current.NextKey.Public())
	if err != nil {
		next.Close()
		return // the next keepalive tries again
	}
	// Only the key changes: the hub's address, its name and its announced
	// next address stay.
	renewed := current
	renewed.Key, renewed.NextKey, renewed.Expires = current.NextKey, tunnel.PrivateKey{}, confirmed.Expires
	if l.save(renewed) != nil {
		// The hub has the new key and the file has both: a restart confirms again.
		next.Close()
		return
	}
	l.replaceTunnel(next, renewed).Close()
}

// step is one request of a renewal on the enrolment port, with its own time limit.
func step(ctx context.Context, sender *tunnel.Device, key tunnel.DeviceKey) (invites.EnrolReply, error) {
	ctx, cancel := context.WithTimeout(ctx, renewStepLimit)
	defer cancel()
	return enrol(ctx, sender, key)
}
