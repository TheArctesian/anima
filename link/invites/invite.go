// Package invites lets a new device join a hub (LNK-003). An invite is the
// hub address and a code of 128 random bits. The code gives a temporary key:
// the hub keeps only the public half of it, never the code, and admits that
// key to the enrolment port only, once, for 15 minutes.
package invites

import (
	"crypto/hkdf"
	"crypto/sha256"
	"encoding/base64"
	"io"
	"strings"

	"github.com/TheArctesian/anima/link/redacted"
	"github.com/TheArctesian/anima/link/tunnel"
)

const (
	codeLength = 16 // 128 bits
	textPrefix = "anima-invite.1."
	keyLabel   = "anima-link invite: temporary key, version 1"
)

// Invite is what the operator gives to a new device. It is a secret until
// it is used or expires.
type Invite struct {
	redacted.Secret
	Hub  tunnel.Address
	Code [codeLength]byte
}

// newInvite makes an invite for a hub with a code from the random source.
func newInvite(hub tunnel.Address, random io.Reader) (Invite, error) {
	invite := Invite{Hub: hub}
	if _, err := io.ReadFull(random, invite.Code[:]); err != nil {
		return Invite{}, err
	}
	return invite, nil
}

// Parse reads the text that Reveal gives.
func Parse(text string) (Invite, error) {
	rest, found := strings.CutPrefix(strings.TrimSpace(text), textPrefix)
	codeText, hubText, split := strings.Cut(rest, ".")
	code, err := base64.RawURLEncoding.DecodeString(codeText)
	if !found || !split || err != nil || len(code) != codeLength {
		return Invite{}, ErrBadInvite
	}
	hub, err := tunnel.ParseAddress(hubText)
	if err != nil {
		return Invite{}, ErrBadInvite
	}
	invite := Invite{Hub: hub}
	copy(invite.Code[:], code)
	return invite, nil
}

// Reveal gives the invite as text, for the operator to pass to the device.
func (i Invite) Reveal() string {
	return textPrefix + base64.RawURLEncoding.EncodeToString(i.Code[:]) + "." + i.Hub.Reveal()
}

// TemporaryKey is the key that the code gives. The device connects with it
// to enrol; the hub knows its public half.
func (i Invite) TemporaryKey() tunnel.PrivateKey {
	seed, err := hkdf.Key(sha256.New, i.Code[:], nil, keyLabel, 32)
	if err != nil {
		panic("invites: HKDF with SHA-256 gives 32 bytes")
	}
	return tunnel.PrivateKeyFromSeed([32]byte(seed))
}
