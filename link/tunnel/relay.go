package tunnel

import (
	"encoding/json"

	"tailscale.com/tailcfg"
)

// Relay is the relay region that a hub uses. The hub address contains it, so
// a hub must save it to keep its address (LNK-001). The zero value means
// "not chosen yet": the hub then takes the nearest region of the default map.
type Relay struct{ region *tailcfg.DERPRegion }

// IsZero reports whether no relay was chosen.
func (r Relay) IsZero() bool { return r.region == nil }

// Name is the relay's name for the operator, or "" when no relay was chosen.
func (r Relay) Name() string {
	if r.region == nil {
		return ""
	}
	return r.region.RegionCode
}

// MarshalJSON gives the form that a state file holds.
func (r Relay) MarshalJSON() ([]byte, error) { return json.Marshal(r.region) }

// UnmarshalJSON reads the form that MarshalJSON gives.
func (r *Relay) UnmarshalJSON(text []byte) error {
	var region *tailcfg.DERPRegion
	if err := json.Unmarshal(text, &region); err != nil {
		return err
	}
	r.region = region
	return nil
}
