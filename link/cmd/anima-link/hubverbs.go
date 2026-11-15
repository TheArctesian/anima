package main

import (
	"context"
	"encoding/json"
	"errors"
	"time"

	"github.com/TheArctesian/anima/link/control"
	"github.com/TheArctesian/anima/link/devices"
	"github.com/TheArctesian/anima/link/hublink"
	"github.com/TheArctesian/anima/link/tunnel"
)

// hubStatus is the answer of the "status" verb.
type hubStatus struct {
	Role    string `json:"role"`
	Relay   string `json:"relay"`
	Devices int    `json:"devices"`
}

// listedDevice is one line of the "devices.list" answer.
type listedDevice struct {
	Key tunnel.DeviceKey `json:"key"`
	// Fingerprint is the short form of Key that "anima-link join" prints
	// on the device, so that the operator can compare the two.
	Fingerprint string    `json:"fingerprint"`
	Name        string    `json:"name"`
	Permitted   time.Time `json:"permitted"`
	Expires     time.Time `json:"expires"`
	Streams     int       `json:"streams"`
	// NextFingerprint is set while the device renews its key: the short
	// form of the key that will replace Key (LNK-005).
	NextFingerprint string `json:"nextFingerprint,omitempty"`
	// Path is "direct", "relayed" or "none" (LNK-006).
	Path string `json:"path"`
	// Relay is the relay's name when the path is relayed.
	Relay string `json:"relay,omitempty"`
}

// revokeRequest is the arguments of "devices.revoke".
type revokeRequest struct {
	Key tunnel.DeviceKey `json:"key"`
}

// revoked is the answer of "devices.revoke".
type revoked struct {
	StreamsClosed int `json:"streamsClosed"`
}

// permitRequest is the arguments of "devices.permit".
type permitRequest struct {
	Key  tunnel.DeviceKey `json:"key"`
	Name string           `json:"name"`
}

func hubVerbs(link *hublink.Link, permitted *devices.Store) map[string]control.Verb {
	return map[string]control.Verb{
		"status": func(context.Context, json.RawMessage) (any, error) {
			return hubStatus{Role: "hub", Relay: link.Relay().Name(), Devices: permitted.Count()}, nil
		},
		// The answer is the hub address: a secret. It goes only to the caller
		// of the control socket, never to a log (LNK-008).
		"address.show": func(context.Context, json.RawMessage) (any, error) {
			return link.Address().Reveal(), nil
		},
		"devices.list": func(context.Context, json.RawMessage) (any, error) {
			listed := []listedDevice{}
			for _, device := range permitted.All() {
				path := link.Path(device.Key)
				next := ""
				if !device.NextKey.IsZero() {
					next = device.NextKey.Fingerprint()
				}
				listed = append(listed, listedDevice{
					NextFingerprint: next,
					Key:             device.Key, Fingerprint: device.Key.Fingerprint(), Name: device.Name, Permitted: device.Permitted, Expires: device.Expires,
					Streams: link.Streams(device.Key), Path: string(path.Kind), Relay: path.Relay,
				})
			}
			return listed, nil
		},
		"devices.permit": func(_ context.Context, args json.RawMessage) (any, error) {
			var request permitRequest
			if err := control.Arguments(args, &request); err != nil {
				return nil, err
			}
			_, err := permitted.Permit(devices.Device{Key: request.Key, Name: request.Name})
			return nil, refusalOf(err)
		},
		"devices.revoke": func(_ context.Context, args json.RawMessage) (any, error) {
			var request revokeRequest
			if err := control.Arguments(args, &request); err != nil {
				return nil, err
			}
			closed, err := link.Revoke(request.Key)
			return revoked{closed}, refusalOf(err)
		},
	}
}

// refusalOf passes a rule of the device list or of the invites to the caller of the control
// socket with its code. Other errors stay inside.
func refusalOf(err error) error {
	var rule interface {
		error
		Code() string
	}
	if errors.As(err, &rule) {
		return &control.Error{Code: rule.Code(), Message: rule.Error()}
	}
	return err
}
