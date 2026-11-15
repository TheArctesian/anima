package devicelink

import (
	"context"
	"time"

	"github.com/TheArctesian/anima/link/devices"
	"github.com/TheArctesian/anima/link/identity"
	"github.com/TheArctesian/anima/link/tunnel"
)

const renewLimit = 15 * time.Second

// renewIfDue replaces the device's key in the last part of its life, with
// no action by the user (LNK-005). The steps keep one working key at each
// moment, also when the device stops between them:
//
//  1. make the next key and save it;
//  2. give its public half to the hub, over the link, with the current key;
//  3. save the next key as the current key;
//  4. connect with it. The hub then drops the old key.
//
// After a stop before step 3 the device repeats step 2 with the same next
// key. After a stop after step 3 it only connects.
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
	ctx, cancel := context.WithTimeout(ctx, renewLimit)
	defer cancel()
	reply, err := enrol(ctx, l.tunnel(), current.NextKey.Public())
	if err != nil {
		return // the next keepalive tries again
	}
	renewed := identity.Device{Key: current.NextKey, Hub: current.Hub, Expires: reply.Expires}
	if l.save(renewed) != nil {
		return
	}
	next, err := tunnel.NewDevice(tunnel.DeviceConfig{Key: renewed.Key, Hub: renewed.Hub, DebugLogf: l.debugLogf})
	if err != nil {
		return
	}
	old := l.replaceTunnel(next, renewed)
	old.Close()
}
