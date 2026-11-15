package devices

import (
	"time"

	"github.com/TheArctesian/anima/link/tunnel"
)

// RenewWindow is the last part of a key's life, in which the device can
// give its next key (LNK-005).
const RenewWindow = 30 * 24 * time.Hour

// Renewal has two steps, so that a device always has one key that works.
//
//  1. renewing: the device, connected with its current key, gives a next
//     key. Both keys are permitted until the current key expires.
//  2. promoted: the next key connects for the first time. It becomes the
//     current key with a new KeyLife, and the old key leaves the list.
//
// A device has one entry in the list through both steps, so a revocation
// removes both keys. A device that is cut between the steps repeats step 1
// with the same next key.

// renewing returns the list with a next key for the device that has
// current as its current key.
func (l list) renewing(current, next tunnel.DeviceKey, now time.Time) (list, Device, error) {
	device, known := l[current]
	if !known || !now.Before(device.Expires) {
		return nil, Device{}, ErrUnknown
	}
	if now.Before(device.Expires.Add(-RenewWindow)) {
		return nil, Device{}, ErrNotDue
	}
	if next.IsZero() {
		return nil, Device{}, ErrNoKey
	}
	if holder, taken := l.holder(next); taken && (holder.Key != current || next == current) {
		return nil, Device{}, ErrKnown
	}
	device.NextKey = next
	updated := l.copy()
	updated[current] = device
	return updated, device, nil
}

// promoted returns the list in which a next key has become the current key
// of its device, and the key that it replaced. It reports false when the
// key is not a next key.
func (l list) promoted(next tunnel.DeviceKey, now time.Time) (list, tunnel.DeviceKey, bool) {
	device, known := l.holder(next)
	if !known || device.NextKey != next || !now.Before(device.Expires) {
		return nil, tunnel.DeviceKey{}, false
	}
	old := device.Key
	device.Key, device.NextKey = next, tunnel.DeviceKey{}
	device.Permitted, device.Expires = now, now.Add(KeyLife)
	updated := l.copy()
	delete(updated, old)
	updated[next] = device
	return updated, old, true
}
