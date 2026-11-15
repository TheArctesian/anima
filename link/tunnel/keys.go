// Package tunnel is the encrypted transport of the link. It is the only
// package that imports the Tailcat library (docs/one-click-plan.md, S1).
package tunnel

import (
	"crypto/sha256"
	"encoding/hex"

	"github.com/tailscale/tailcat"
	"tailscale.com/types/key"
)

// PrivateKey is the secret identity of a hub or a device. Its text form is a
// secret (LNK-008).
type PrivateKey struct{ node key.NodePrivate }

// NewPrivateKey makes a new identity.
func NewPrivateKey() PrivateKey { return PrivateKey{key.NewNode()} }

// IsZero reports whether the key was never set.
func (k PrivateKey) IsZero() bool { return k.node.IsZero() }

// Public returns the key that a hub puts in its list of permitted devices.
func (k PrivateKey) Public() DeviceKey { return DeviceKey{k.node.Public()} }

// MarshalText gives the form that a state file holds.
func (k PrivateKey) MarshalText() ([]byte, error) { return k.node.MarshalText() }

// UnmarshalText reads the form that MarshalText gives.
func (k *PrivateKey) UnmarshalText(text []byte) error { return k.node.UnmarshalText(text) }

// DeviceKey is the public key of a hub or a device. It is not a secret, but
// logs must use Fingerprint and not the key (LNK-008).
type DeviceKey struct{ node key.NodePublic }

// IsZero reports whether the key was never set.
func (k DeviceKey) IsZero() bool { return k.node.IsZero() }

// Fingerprint is a short name for the key that is safe in a log.
func (k DeviceKey) Fingerprint() string {
	sum := sha256.Sum256(k.node.AppendTo(nil))
	return hex.EncodeToString(sum[:4])
}

// MarshalText gives the form that the list of permitted devices holds.
func (k DeviceKey) MarshalText() ([]byte, error) { return k.node.MarshalText() }

// UnmarshalText reads the form that MarshalText gives.
func (k *DeviceKey) UnmarshalText(text []byte) error { return k.node.UnmarshalText(text) }

// PresharedKey is the second secret of a hub. It is part of the hub address.
type PresharedKey struct{ psk tailcat.PresharedKey }

// NewPresharedKey makes a new random pre-shared key.
func NewPresharedKey() PresharedKey { return PresharedKey{tailcat.NewPresharedKey()} }

// IsZero reports whether the key was never set.
func (k PresharedKey) IsZero() bool { return k.psk.IsZero() }

// MarshalText gives the form that a state file holds.
func (k PresharedKey) MarshalText() ([]byte, error) { return k.psk.MarshalText() }

// UnmarshalText reads the form that MarshalText gives.
func (k *PresharedKey) UnmarshalText(text []byte) error { return k.psk.UnmarshalText(text) }

// Address is what a device needs to find and authenticate a hub: the hub's
// public key, its pre-shared key and its relay. It is a secret (LNK-008).
type Address string
