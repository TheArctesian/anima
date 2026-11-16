package hublink

import (
	"time"

	"github.com/TheArctesian/anima/link/tunnel"
)

// Revoke removes a device from the list and ends its open sessions
// (LNK-004). It returns how many streams it closed. The order matters: the
// key leaves the list first, so that the device cannot connect again; then
// its streams are closed, so that the device sees its sessions end; then the
// tunnel forgets it. If the hub link stops between the steps, the next
// sweep after a restart does the rest.
func (l *Link) Revoke(device tunnel.DeviceKey) (closed int, err error) {
	revoked, err := l.devices.Revoke(device)
	if err != nil {
		return 0, err
	}
	closed = l.disconnect(revoked.Key)
	if !revoked.NextKey.IsZero() {
		closed += l.disconnect(revoked.NextKey)
	}
	l.log.Info("a device was revoked", "device", revoked.Key, "name", revoked.Name, "streams", closed)
	return closed, nil
}

// disconnect closes the streams of a key that is not admitted any more, and
// makes the tunnel forget the key a moment later. The tunnel's own call
// does not close streams: without the first step the sessions would stay
// open with no traffic. The wait lets the close reach the device; this call
// does not wait for it.
func (l *Link) disconnect(key tunnel.DeviceKey) (closed int) {
	closed = l.streams.closeOf(key)
	time.AfterFunc(closeGrace, func() {
		select {
		case <-l.stopped:
		default:
			l.hub.Disconnect(key)
		}
	})
	return closed
}

// sweep disconnects each device that the tunnel still knows but that is
// not admitted any more: its key expired, its invite ended, or a revocation
// did not reach its disconnect.
func (l *Link) sweep(every time.Duration) {
	ticker := time.NewTicker(every)
	defer ticker.Stop()
	for {
		select {
		case <-l.stopped:
			return
		case <-ticker.C:
		}
		for key, path := range l.hub.Paths() {
			connected := path.Kind != tunnel.PathNone || l.streams.count(key) > 0
			if connected && !l.admitted(key) {
				l.log.Info("a device that is not permitted any more was disconnected", "device", key)
				l.disconnect(key)
			}
		}
	}
}
