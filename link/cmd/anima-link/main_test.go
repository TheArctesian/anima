package main

import (
	"bytes"
	"strings"
	"testing"
)

func TestRunRefusesAMissingOrUnknownRole(t *testing.T) {
	for _, args := range [][]string{nil, {"serve"}} {
		var stderr bytes.Buffer
		if code := run(args, &stderr); code != 2 {
			t.Errorf("run(%v) = %d, want 2", args, code)
		}
		if !strings.Contains(stderr.String(), usage) {
			t.Errorf("run(%v) did not print the usage: %q", args, stderr.String())
		}
	}
}
