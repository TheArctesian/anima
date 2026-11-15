package invites

import "github.com/TheArctesian/anima/link/tunnel"

// EnrolRequest is the one line that a device sends on the enrolment port.
type EnrolRequest struct {
	// Key is the device's own new public key.
	Key tunnel.DeviceKey `json:"key"`
}

// EnrolReply is the one line that the hub link answers.
type EnrolReply struct {
	OK bool `json:"ok"`
	// Code names the rule that refused the device, for a program.
	Code string `json:"code,omitempty"`
}
