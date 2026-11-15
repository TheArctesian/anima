package invites

import (
	"time"

	"github.com/TheArctesian/anima/link/tunnel"
)

// EnrolRequest is the one line that a device sends on the enrolment port.
// A device that connects with the temporary key of an invite joins the hub
// (LNK-003). A device that connects with its current key gives its next key
// (LNK-005). A device that connects with its next key confirms the renewal.
type EnrolRequest struct {
	// Key is the device's own new public key. In a confirmation it is the
	// key that sends, and the hub does not read it.
	Key tunnel.DeviceKey `json:"key"`
}

// Notice is the one line that the hub link writes on the notice port.
type Notice struct {
	// NextHub is the address that the hub will have after its relay change,
	// as text. It is empty when no change is planned. It is a secret.
	NextHub string `json:"nextHub,omitempty"`
}

// EnrolReply is the one line that the hub link answers.
type EnrolReply struct {
	OK bool `json:"ok"`
	// Expires is when the device's key stops being permitted. A join and a
	// confirmation give it. The device renews before it.
	Expires time.Time `json:"expires,omitzero"`
	// Code names the rule that refused the device, for a program.
	Code string `json:"code,omitempty"`
}
