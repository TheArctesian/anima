package hublink

import (
	"bufio"
	"encoding/json"
	"net"
	"time"

	"github.com/TheArctesian/anima/link/devices"
	"github.com/TheArctesian/anima/link/invites"
	"github.com/TheArctesian/anima/link/tunnel"
)

// Enrolment is what a hub link needs to let devices join with an invite
// (LNK-003).
type Enrolment struct {
	Invites *invites.Store
	Devices *devices.Store
	// Now is the clock.
	Now func() time.Time
}

const (
	enrolLimit       = 10 * time.Second
	enrolRequestSize = 1024
)

// admits is the tunnel's question: a permitted device, or the temporary key
// of an open invite.
func (l *Link) admits(key tunnel.DeviceKey) bool {
	if l.permits(key) {
		return true
	}
	return l.enrolment != nil && l.enrolment.Invites.Admits(key, l.enrolment.Now())
}

func (l *Link) acceptEnrolments(listener net.Listener) {
	defer close(l.enrolled)
	for {
		stream, err := listener.Accept()
		if err != nil {
			return
		}
		go l.enrol(stream)
	}
}

// enrol serves one stream of the enrolment port. The stream's key is the
// temporary key of an invite. The invite is ended before anything else
// happens, so a second use fails also when this use fails later.
func (l *Link) enrol(stream net.Conn) {
	defer stream.Close()
	temporary, known := l.hub.DeviceOf(stream.RemoteAddr())
	if !known {
		return
	}
	_ = stream.SetDeadline(time.Now().Add(enrolLimit))
	var request invites.EnrolRequest
	line, err := bufio.NewReader(&limited{stream, enrolRequestSize}).ReadBytes('\n')
	if err != nil || json.Unmarshal(line, &request) != nil {
		return
	}
	now := l.enrolment.Now()
	invite, err := l.enrolment.Invites.Use(temporary, now)
	if err != nil {
		return
	}
	reply := invites.EnrolReply{OK: true}
	err = l.enrolment.Devices.Permit(devices.Device{Key: request.Key, Name: invite.Name, Permitted: now.UTC()})
	if err != nil {
		reply = invites.EnrolReply{Code: "refused"}
		if rule, isRule := err.(interface{ Code() string }); isRule {
			reply.Code = rule.Code()
		}
		l.log.Warn("an invite was used, but the device was not permitted", "name", invite.Name, "cause", reply.Code)
	} else {
		l.log.Info("a device joined with an invite", "device", request.Key, "name", invite.Name)
	}
	_ = json.NewEncoder(stream).Encode(reply)
	// The temporary key has done its work. Let the answer reach the device,
	// then make the tunnel forget the key.
	stream.Close()
	time.Sleep(closeGrace)
	l.hub.Disconnect(temporary)
}

// limited stops a read after a number of bytes.
type limited struct {
	net.Conn
	left int
}

func (l *limited) Read(p []byte) (int, error) {
	if l.left <= 0 {
		return 0, net.ErrClosed
	}
	if len(p) > l.left {
		p = p[:l.left]
	}
	n, err := l.Conn.Read(p)
	l.left -= n
	return n, err
}
