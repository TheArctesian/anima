package main

import (
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"testing"

	"github.com/TheArctesian/anima/link/devicelink"
	"github.com/TheArctesian/anima/link/statedir"
)

func TestM31TheCodesAreTheDesktopLinkCodes(t *testing.T) {
	cases := map[error]string{
		errBadInvite:           "bad-invite",
		devicelink.ErrNoAnswer: "no-answer",
		fmt.Errorf("join: %w", devicelink.ErrRefused): "refused",
		devicelink.ErrNotEnrolled:                     "not-joined",
		statedir.ErrInUse:                             "folder",
		errors.New("anything else"):                   "failed",
	}
	for err, want := range cases {
		if got := code(err); got != want {
			t.Errorf("code(%v) = %q, want %q", err, got, want)
		}
	}
}

func TestM31AJoinWithTextThatIsNotAnInviteChangesNothing(t *testing.T) {
	state := filepath.Join(t.TempDir(), "link")
	if _, err := join(state, "not an invite"); !errors.Is(err, errBadInvite) {
		t.Fatalf("join = %v, want errBadInvite", err)
	}
	if _, err := os.Stat(state); !errors.Is(err, os.ErrNotExist) {
		t.Fatalf("the state directory was made: %v", err)
	}
}

func TestM31ADeviceWithNoHubCannotStartAndNothingRuns(t *testing.T) {
	state := filepath.Join(t.TempDir(), "link")
	if _, err := start(state, "127.0.0.1:0"); code(err) != "not-joined" {
		t.Fatalf("start = %v, want not-joined", err)
	}
	if got := status(); got.Running {
		t.Fatalf("status = %+v, want not running", got)
	}
	stop()
}
