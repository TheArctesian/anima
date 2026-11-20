//go:build unix

package unit

import (
	"bytes"
	"context"
	"errors"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// recorded is a Runner that runs nothing: it notes each step and answers
// from a list, then with success.
type recorded struct {
	ran     []string
	answers []bool
}

func (r *recorded) run(_ context.Context, step Step) (bool, error) {
	r.ran = append(r.ran, strings.Join(step.Args, " "))
	if len(r.answers) == 0 {
		return true, nil
	}
	answer := r.answers[0]
	r.answers = r.answers[1:]
	return answer, nil
}

func planIn(t *testing.T) Plan {
	t.Helper()
	plan, err := PlanFor(settings, System{GOOS: "darwin", UID: 501, Home: t.TempDir()})
	if err != nil {
		t.Fatal(err)
	}
	plan.Start[1].RetryAfter = 1
	return plan
}

func TestHMD058InstallWritesTheFileThenStartsTheUnitAndRemoveTakesBothAway(t *testing.T) {
	plan := planIn(t)
	runner := &recorded{}
	if err := Install(context.Background(), plan, runner.run); err != nil {
		t.Fatal(err)
	}
	written, err := os.ReadFile(plan.File)
	if err != nil || string(written) != plan.Content {
		t.Fatalf("the file of the unit: %v", err)
	}
	if len(runner.ran) != 2 || !strings.HasPrefix(runner.ran[1], "bootstrap ") || !Installed(plan) {
		t.Fatalf("ran %v, installed %v", runner.ran, Installed(plan))
	}

	runner = &recorded{}
	if err := Remove(context.Background(), plan, runner.run); err != nil {
		t.Fatal(err)
	}
	if Installed(plan) || len(runner.ran) != 1 || !strings.HasPrefix(runner.ran[0], "bootout ") {
		t.Fatalf("after remove: installed %v, ran %v", Installed(plan), runner.ran)
	}
	if err := Remove(context.Background(), plan, (&recorded{}).run); err != nil {
		t.Errorf("a second remove: %v, want no error", err)
	}
}

func TestAUnitThatIsNotLoadedMayFailToStopAndTheStartIsTriedOnceMore(t *testing.T) {
	plan := planIn(t)
	// The bootout fails (nothing is loaded), the first bootstrap fails, the second passes.
	runner := &recorded{answers: []bool{false, false, true}}
	if err := Install(context.Background(), plan, runner.run); err != nil {
		t.Fatalf("install: %v", err)
	}
	if len(runner.ran) != 3 {
		t.Errorf("ran %v, want the bootstrap twice", runner.ran)
	}

	runner = &recorded{answers: []bool{true, false, false}}
	if err := Install(context.Background(), plan, runner.run); err == nil || errors.Is(err, ErrRefused) {
		t.Errorf("a start that fails twice: err = %v, want a failure that names the command", err)
	}
}

func TestHMD058AProgramThatOtherUsersCanReplaceIsRefused(t *testing.T) {
	folder := t.TempDir()
	program := filepath.Join(folder, "anima-link")
	if err := os.WriteFile(program, []byte("#!/bin/sh\n"), 0o755); err != nil {
		t.Fatal(err)
	}
	uid := os.Getuid()
	if err := CheckProgram(program, uid); err != nil {
		t.Fatalf("a program of this user in a folder of this user: %v", err)
	}
	refusedAfter := func(what string, path string, mode os.FileMode) {
		t.Helper()
		if err := os.Chmod(path, mode); err != nil {
			t.Fatal(err)
		}
		if err := CheckProgram(program, uid); !errors.Is(err, ErrRefused) {
			t.Errorf("%s: err = %v, want ErrRefused", what, err)
		}
	}
	refusedAfter("a program that its group can write", program, 0o775)
	refusedAfter("a program that its owner cannot run", program, 0o644)
	if err := os.Chmod(program, 0o755); err != nil {
		t.Fatal(err)
	}
	refusedAfter("a folder that others can write", folder, 0o777)
	if err := os.Chmod(folder, 0o755); err != nil {
		t.Fatal(err)
	}
	if err := CheckProgram(program, uid+1); !errors.Is(err, ErrRefused) {
		t.Errorf("a program of a different user: err = %v, want ErrRefused", err)
	}
	if err := CheckProgram(filepath.Join(folder, "absent"), uid); !errors.Is(err, ErrRefused) {
		t.Errorf("an absent program: err = %v, want ErrRefused", err)
	}
}

func TestTheCommandShowsWhetherTheUnitIsInstalledAndRefusesAWrongUse(t *testing.T) {
	home := t.TempDir()
	t.Setenv("HOME", home)
	t.Setenv("XDG_CONFIG_HOME", "")
	t.Setenv("XDG_RUNTIME_DIR", "")
	var out bytes.Buffer
	if err := Command(context.Background(), []string{"show"}, &out); err != nil {
		t.Fatal(err)
	}
	if !strings.HasPrefix(out.String(), `{"installed":false,"file":"`+home) {
		t.Errorf("show = %q", out.String())
	}
	for _, args := range [][]string{
		nil, {"sideways"}, {"show", "more"},
		{"install"}, {"install", "-state", "/s"}, {"install", "-state", "/s", "-listen", "0.0.0.0:41001"},
		{"install", "-state", "/s", "-listen", "127.0.0.1:0"}, {"install", "-state", "relative", "-listen", "127.0.0.1:41001"},
	} {
		if err := Command(context.Background(), args, &out); !errors.Is(err, ErrRefused) {
			t.Errorf("unit %v: err = %v, want ErrRefused", args, err)
		}
	}
}
