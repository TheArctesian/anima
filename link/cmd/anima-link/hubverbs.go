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
	Key       tunnel.DeviceKey `json:"key"`
	Name      string           `json:"name"`
	Permitted time.Time        `json:"permitted"`
	Streams   int              `json:"streams"`
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

func hubVerbs(link *hublink.Link, permitted *devices.Store, now func() time.Time) map[string]control.Verb {
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
				listed = append(listed, listedDevice{device.Key, device.Name, device.Permitted, link.Streams(device.Key)})
			}
			return listed, nil
		},
		"devices.permit": func(_ context.Context, args json.RawMessage) (any, error) {
			var request permitRequest
			if err := control.Arguments(args, &request); err != nil {
				return nil, err
			}
			err := permitted.Permit(devices.Device{Key: request.Key, Name: request.Name, Permitted: now().UTC()})
			return nil, refusalOf(err)
		},
		"devices.revoke": func(_ context.Context, args json.RawMessage) (any, error) {
			var request revokeRequest
			if err := control.Arguments(args, &request); err != nil {
				return nil, err
			}
			if err := permitted.Revoke(request.Key); err != nil {
				return nil, refusalOf(err)
			}
			closed, err := link.Disconnect(request.Key)
			return revoked{closed}, err
		},
	}
}

// refusalOf passes a rule of the device list to the caller of the control
// socket with its code. Other errors stay inside.
func refusalOf(err error) error {
	var rule *devices.Refusal
	if errors.As(err, &rule) {
		return &control.Error{Code: rule.Code, Message: rule.Error()}
	}
	return err
}
