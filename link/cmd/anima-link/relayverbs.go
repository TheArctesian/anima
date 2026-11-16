package main

import (
	"context"
	"encoding/json"
	"errors"

	"github.com/TheArctesian/anima/link/control"
	"github.com/TheArctesian/anima/link/hublink"
	"github.com/TheArctesian/anima/link/identity"
	"github.com/TheArctesian/anima/link/statedir"
	"github.com/TheArctesian/anima/link/tunnel"
)

// relayStatus is the answer of "relay.show".
type relayStatus struct {
	Relay string `json:"relay"`
	// Next is the relay that the hub has announced, or empty.
	Next string `json:"next,omitempty"`
}

// relayRequest is the arguments of "relay.announce".
type relayRequest struct {
	// Relay is "host" or "host:port".
	Relay string `json:"relay"`
}

var errNoNextRelay = &control.Error{Code: "no_next_relay", Message: "no relay is announced; use relay.announce first"}

// relayVerbs are the two steps of a relay change (LNK-007): relay.announce
// tells the connected devices the hub's next address; relay.switch makes the
// next relay the relay and stops the hub link, which starts on it again.
func relayVerbs(link *hublink.Link, dir *statedir.Dir, stop context.CancelFunc) map[string]control.Verb {
	return map[string]control.Verb{
		"relay.show": func(context.Context, json.RawMessage) (any, error) {
			hub, err := identity.LoadHub(dir)
			return relayStatus{Relay: link.Relay().Name(), Next: hub.NextRelay.Name()}, err
		},
		"relay.announce": func(_ context.Context, args json.RawMessage) (any, error) {
			var request relayRequest
			if err := control.Arguments(args, &request); err != nil {
				return nil, err
			}
			next, err := tunnel.OwnRelay(request.Relay)
			if errors.Is(err, tunnel.ErrBadRelay) {
				return nil, &control.Error{Code: "bad_relay", Message: err.Error()}
			}
			hub, err := identity.LoadHub(dir)
			if err != nil {
				return nil, err
			}
			hub.NextRelay = next
			if err := hub.Save(dir); err != nil {
				return nil, err
			}
			link.Announce(next)
			return relayStatus{Relay: link.Relay().Name(), Next: next.Name()}, nil
		},
		"relay.switch": func(context.Context, json.RawMessage) (any, error) {
			hub, err := identity.LoadHub(dir)
			if err != nil {
				return nil, err
			}
			switched, err := hub.Switched()
			if errors.Is(err, identity.ErrNoNextRelay) {
				return nil, errNoNextRelay
			}
			if err := switched.Save(dir); err != nil {
				return nil, err
			}
			// The server sends this reply before it stops.
			stop()
			return relayStatus{Relay: switched.Relay.Name()}, nil
		},
	}
}
