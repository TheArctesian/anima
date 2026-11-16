// Package redacted gives the secrets of the link one behaviour: no way of
// printing, logging or encoding shows them (LNK-008).
package redacted

import (
	"fmt"
	"log/slog"
)

// Text is what a secret shows as.
const Text = "[secret]"

// Secret is put in a struct that holds a secret, as an embedded field. The
// struct then shows as Text for every fmt verb, in structured logs, in JSON
// and in every other text encoding. The struct gives its text only by a
// method of its own, by convention named Reveal.
//
// Two cases are not covered: a printer that reads memory by reflection, and
// fmt on a struct that holds a secret in an unexported field (fmt cannot
// call the methods of such a field). Keep secrets in exported fields.
type Secret struct{}

// String hides the secret.
func (Secret) String() string { return Text }

// GoString hides the secret.
func (Secret) GoString() string { return Text }

// Format hides the secret for every verb, also %d and %x.
func (Secret) Format(state fmt.State, _ rune) { fmt.Fprint(state, Text) }

// LogValue hides the secret in structured logs.
func (Secret) LogValue() slog.Value { return slog.StringValue(Text) }

// MarshalText hides the secret in JSON and in every other text encoding.
func (Secret) MarshalText() ([]byte, error) { return []byte(Text), nil }
