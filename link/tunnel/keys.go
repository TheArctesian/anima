// Package tunnel is the encrypted transport of the link. It is the only
// package that imports the Tailcat library (docs/one-click-plan.md, S1).
package tunnel

import (
	"crypto/sha256"
	"encoding/hex"
	"errors"
	"fmt"
	"log/slog"

	"github.com/tailscale/tailcat"
	"tailscale.com/types/key"
)

// ErrBadKey means that a text is not a key. The error never holds the text,
// because the text can be a damaged secret (LNK-008).
var ErrBadKey = errors.New("tunnel: the text is not a key")

// secret is what PrivateKey, PresharedKey and Address have in common: no
// way of printing, logging or encoding shows them (LNK-008). Only Reveal
// gives the text. Two cases are not covered: a printer that reads memory by
// reflection, and fmt on a struct that holds a secret in an unexported
// field (fmt cannot call the methods of such a field). Keep secrets in
// exported fields.
type secret struct{}

const redacted = "[secret]"

// String hides the secret.
func (secret) String() string { return redacted }

// GoString hides the secret.
func (secret) GoString() string { return redacted }

// Format hides the secret for every verb, also %d and %x.
func (secret) Format(state fmt.State, _ rune) { fmt.Fprint(state, redacted) }

// LogValue hides the secret in structured logs.
func (secret) LogValue() slog.Value { return slog.StringValue(redacted) }

// MarshalText hides the secret in JSON and in every other text encoding.
func (secret) MarshalText() ([]byte, error) { return []byte(redacted), nil }

// PrivateKey is the secret identity of a hub or a device.
type PrivateKey struct {
	secret
	node key.NodePrivate
}

// NewPrivateKey makes a new identity.
func NewPrivateKey() PrivateKey { return PrivateKey{node: key.NewNode()} }

// ParsePrivateKey reads the text that Reveal gives.
func ParsePrivateKey(text string) (PrivateKey, error) {
	var k PrivateKey
	if k.node.UnmarshalText([]byte(text)) != nil || k.node.IsZero() {
		return PrivateKey{}, ErrBadKey
	}
	return k, nil
}

// Reveal gives the key as text, for the state file only.
func (k PrivateKey) Reveal() string {
	text, _ := k.node.MarshalText() // never fails for a key
	return string(text)
}

// IsZero reports whether the key was never set.
func (k PrivateKey) IsZero() bool { return k.node.IsZero() }

// Public returns the key that a hub puts in its list of permitted devices.
func (k PrivateKey) Public() DeviceKey { return DeviceKey{k.node.Public()} }

// DeviceKey is the public key of a hub or a device. It is not a secret, but
// it prints and logs as its fingerprint (LNK-008). Its JSON form is the key.
type DeviceKey struct{ node key.NodePublic }

// IsZero reports whether the key was never set.
func (k DeviceKey) IsZero() bool { return k.node.IsZero() }

// Fingerprint is a short name for the key that is safe in a log.
func (k DeviceKey) Fingerprint() string {
	sum := sha256.Sum256(k.node.AppendTo(nil))
	return hex.EncodeToString(sum[:4])
}

// String is the fingerprint.
func (k DeviceKey) String() string { return k.Fingerprint() }

// LogValue is the fingerprint.
func (k DeviceKey) LogValue() slog.Value { return slog.StringValue(k.Fingerprint()) }

// MarshalText gives the form that the list of permitted devices holds.
func (k DeviceKey) MarshalText() ([]byte, error) { return k.node.MarshalText() }

// UnmarshalText reads the form that MarshalText gives.
func (k *DeviceKey) UnmarshalText(text []byte) error {
	if k.node.UnmarshalText(text) != nil {
		return ErrBadKey
	}
	return nil
}

// PresharedKey is the second secret of a hub. It is part of the hub address.
type PresharedKey struct {
	secret
	psk tailcat.PresharedKey
}

// NewPresharedKey makes a new random pre-shared key.
func NewPresharedKey() PresharedKey { return PresharedKey{psk: tailcat.NewPresharedKey()} }

// ParsePresharedKey reads the text that Reveal gives.
func ParsePresharedKey(text string) (PresharedKey, error) {
	var k PresharedKey
	if k.psk.UnmarshalText([]byte(text)) != nil || k.psk.IsZero() {
		return PresharedKey{}, ErrBadKey
	}
	return k, nil
}

// Reveal gives the key as text, for the state file only.
func (k PresharedKey) Reveal() string {
	text, _ := k.psk.MarshalText() // never fails for a key
	return string(text)
}

// IsZero reports whether the key was never set.
func (k PresharedKey) IsZero() bool { return k.psk.IsZero() }

// Address is what a device needs to find and authenticate a hub: the hub's
// public key, its pre-shared key and its relay.
type Address struct {
	secret
	text tailcat.Addr
}

// ParseAddress reads the text that Reveal gives.
func ParseAddress(text string) (Address, error) {
	if _, err := tailcat.ParseAddr(tailcat.Addr(text)); err != nil {
		// The cause is dropped: it can quote the address.
		return Address{}, ErrBadAddress
	}
	return Address{text: tailcat.Addr(text)}, nil
}

// Reveal gives the address as text: for the state file, and for the operator
// who gives it to a device.
func (a Address) Reveal() string { return string(a.text) }

// IsZero reports whether the address was never set.
func (a Address) IsZero() bool { return a.text == "" }
