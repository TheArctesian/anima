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

// permitRequest is the arguments of "devices.permit".
type permitRequest struct {
	Key  tunnel.DeviceKey `json:"key"`
	Name string           `json:"name"`
}

func hubVerbs(link *hublink.Link, permitted *devices.Store, now func() time.Time) map[string]control.Verb {
	return map[string]control.Verb{
		"status": func(context.Context, json.RawMessage) (any, error) {
			return hubStatus{Role: "hub", Relay: link.Relay().Name(), Devices: len(permitted.All())}, nil
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
			if json.Unmarshal(args, &request) != nil {
				return nil, &control.Error{Code: control.CodeBadRequest, Message: `devices.permit needs {"key": "nodekey:...", "name": "..."}`}
			}
			err := permitted.Permit(devices.Device{Key: request.Key, Name: request.Name, Permitted: now().UTC()})
			return nil, deviceRefusal(err)
		},
	}
}

// deviceRefusal turns a rule of the device list into a refusal that the
// caller can read. Other errors stay inside.
func deviceRefusal(err error) error {
	for code, rule := range map[string]error{
		"device_known": devices.ErrKnown, "bad_name": devices.ErrBadName, "no_key": devices.ErrNoKey,
	} {
		if errors.Is(err, rule) {
			return &control.Error{Code: code, Message: rule.Error()}
		}
	}
	return err
}
