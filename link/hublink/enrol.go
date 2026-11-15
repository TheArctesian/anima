package hublink

import (
	"bufio"
	"encoding/json"
	"errors"
	"io"
	"net"
	"time"

	"github.com/TheArctesian/anima/link/devices"
	"github.com/TheArctesian/anima/link/invites"
	"github.com/TheArctesian/anima/link/tunnel"
)

const (
	enrolLimit       = 10 * time.Second
	enrolRequestSize = 1024
	// maxEnrolments is how many enrolments run at one time. More are closed.
	maxEnrolments = 16
)

// admitted reports whether a key can be connected now: a permitted device,
// or the temporary key of an open invite.
func (l *Link) admitted(key tunnel.DeviceKey) bool {
	return l.devices.Permits(key) || (l.invites != nil && l.invites.Admits(key, l.now()))
}

// admits is the tunnel's question when a key connects. When the key is the
// next key of a device, this first connection completes the renewal: the
// next key becomes the device's key and the old key is disconnected
// (LNK-005).
func (l *Link) admits(key tunnel.DeviceKey) bool {
	if !l.admitted(key) {
		return false
	}
	old, promoted, err := l.devices.Promote(key)
	if err != nil {
		l.log.Error("a renewed key could not be saved", "device", key)
	}
	if promoted {
		l.log.Info("a device renewed its key", "device", key, "replaced", old)
		// The tunnel can ask before Start has returned; the disconnect waits.
		go func() {
			<-l.started
			l.disconnect(old)
		}()
	}
	return true
}

// enrol serves one stream of the enrolment port: one request, one reply.
func (l *Link) enrol(stream net.Conn) {
	defer stream.Close()
	sender, known := l.hub.DeviceOf(stream.RemoteAddr())
	if !known || !l.admitted(sender) {
		return
	}
	select {
	case l.enrolling <- struct{}{}:
		defer func() { <-l.enrolling }()
	default:
		return
	}
	l.enrolments.Add(1)
	defer l.enrolments.Done()
	_ = stream.SetDeadline(time.Now().Add(enrolLimit))
	var request invites.EnrolRequest
	line, err := bufio.NewReader(io.LimitReader(stream, enrolRequestSize)).ReadBytes('\n')
	if err != nil || json.Unmarshal(line, &request) != nil {
		return
	}
	if l.devices.Permits(sender) {
		_ = json.NewEncoder(stream).Encode(l.renew(sender, request.Key))
		return
	}
	// From here the sender is the temporary key of an open invite.
	reply, answered := l.join(sender, request.Key)
	if !answered {
		return
	}
	_ = json.NewEncoder(stream).Encode(reply)
	// The temporary key has done its work.
	stream.Close()
	l.disconnect(sender)
}

// join uses the invite whose temporary key sent the request. The invite is
// ended before anything else happens, so a second use fails also when this
// use fails later. With no open invite there is no answer.
func (l *Link) join(temporary, device tunnel.DeviceKey) (reply invites.EnrolReply, answered bool) {
	now := l.now()
	invite, err := l.invites.Use(temporary, now)
	if err != nil {
		return invites.EnrolReply{}, false
	}
	if device == temporary || !l.deviceKey(device) {
		l.log.Warn("an invite was used with a key that cannot be a device key", "name", invite.Name)
		return invites.EnrolReply{Code: codeBadKey}, true
	}
	permitted, err := l.devices.Permit(devices.Device{Key: device, Name: invite.Name, Permitted: now.UTC()})
	if err != nil {
		l.log.Warn("an invite was used, but the device was not permitted", "name", invite.Name, "cause", codeOf(err))
		return invites.EnrolReply{Code: codeOf(err)}, true
	}
	l.log.Info("a device joined with an invite", "device", device, "name", invite.Name)
	return invites.EnrolReply{OK: true, Expires: permitted.Expires}, true
}

// renew records the next key of a permitted device. The reply's expiry time
// is that of the next key if it connects now; the true time starts when it
// first connects, which is not earlier.
func (l *Link) renew(current, next tunnel.DeviceKey) invites.EnrolReply {
	if !l.deviceKey(next) {
		return invites.EnrolReply{Code: codeBadKey}
	}
	device, err := l.devices.Renew(current, next)
	if err != nil {
		return invites.EnrolReply{Code: codeOf(err)}
	}
	l.log.Info("a device gave its next key", "device", current, "name", device.Name)
	return invites.EnrolReply{OK: true, Expires: l.now().UTC().Add(devices.KeyLife)}
}

const codeBadKey = "bad_key"

// deviceKey reports whether a key can become a device's key. The hub's own
// key cannot. The temporary key of an open invite cannot: whoever reads the
// invite text later could make that key, and a used invite would then work
// with no end (LNK-003).
func (l *Link) deviceKey(key tunnel.DeviceKey) bool {
	return key != l.ownKey && !l.invites.Admits(key, l.now())
}

// codeOf is the code of a rule, or "refused" for an error that is not a rule.
func codeOf(err error) string {
	var rule interface {
		error
		Code() string
	}
	if errors.As(err, &rule) {
		return rule.Code()
	}
	return "refused"
}
