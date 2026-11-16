package tunnel

import "time"

// PathKind says how the traffic of one device reaches the hub (LNK-006).
type PathKind string

// The kinds of path.
const (
	PathDirect  PathKind = "direct"
	PathRelayed PathKind = "relayed"
	PathNone    PathKind = "none"
)

// Path is the state of the link between one device and the hub.
type Path struct {
	Kind PathKind
	// Relay is the relay's name. Only the hub knows it.
	Relay string
	// Latency is the round trip of the last ping. Only the device measures it.
	Latency time.Duration
}
