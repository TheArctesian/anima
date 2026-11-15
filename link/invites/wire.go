package invites

import (
	"time"

	"github.com/TheArctesian/anima/link/tunnel"
)

// EnrolRequest is the one line that a device sends on the enrolment port.
// A device that connects with the temporary key of an invite joins the hub
// (LNK-003). A device that connects with its own permitted key renews it
// (LNK-005).
type EnrolRequest struct {
	// Key is the device's own new public key.
	Key tunnel.DeviceKey `json:"key"`
}

// EnrolReply is the one line that the hub link answers.
type EnrolReply struct {
	OK bool `json:"ok"`
	// Expires is when the key in the request stops being permitted. The
	// device renews before it.
	Expires time.Time `json:"expires,omitzero"`
	// Code names the rule that refused the device, for a program.
	Code string `json:"code,omitempty"`
}
