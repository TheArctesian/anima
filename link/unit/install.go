package unit

import (
	"context"
	"errors"
	"fmt"
	"io/fs"
	"os"
	"os/exec"
	"path/filepath"
	"time"
)

// stepDeadline is how long launchctl or systemctl may take.
const stepDeadline = 30 * time.Second

// searchPath is the only search path that a command gets: the environment
// of the caller is not passed on.
const searchPath = "PATH=/usr/sbin:/usr/bin:/sbin:/bin"

// Runner runs one step and reports whether it ended with success. The
// error is for a step that could not run at all.
type Runner func(ctx context.Context, step Step) (bool, error)

// Run is the Runner of the real system.
func Run(ctx context.Context, step Step) (bool, error) {
	ctx, cancel := context.WithTimeout(ctx, stepDeadline)
	defer cancel()
	command := exec.CommandContext(ctx, step.Program, step.Args...)
	command.Env = append([]string{searchPath}, step.Env...)
	command.Dir = "/"
	err := command.Run()
	var exit *exec.ExitError
	if errors.As(err, &exit) && ctx.Err() == nil {
		return false, nil
	}
	return err == nil, err
}

// runAll runs the steps in order and stops at the first that fails and may
// not fail.
func runAll(ctx context.Context, run Runner, steps []Step) error {
	for _, step := range steps {
		ok, err := run(ctx, step)
		if !ok && err == nil && step.RetryAfter > 0 {
			select {
			case <-ctx.Done():
				return ctx.Err()
			case <-time.After(step.RetryAfter):
			}
			ok, err = run(ctx, step)
		}
		if step.MayFail || (ok && err == nil) {
			continue
		}
		if err != nil {
			return fmt.Errorf("unit: %s %v: %w", step.Program, step.Args, err)
		}
		return fmt.Errorf("unit: %s %v failed", step.Program, step.Args)
	}
	return nil
}

// Install writes the file of the unit and starts it. A unit that runs is
// started again, so that it uses the file and the program of now. The
// folder of the file is made when it is absent.
func Install(ctx context.Context, plan Plan, run Runner) error {
	if err := os.MkdirAll(filepath.Dir(plan.File), 0o755); err != nil {
		return err
	}
	// The file is written beside its place and renamed, so that a reader
	// never finds half a file.
	next := plan.File + ".next"
	if err := os.WriteFile(next, []byte(plan.Content), 0o644); err != nil {
		return err
	}
	if err := os.Rename(next, plan.File); err != nil {
		_ = os.Remove(next)
		return err
	}
	return runAll(ctx, run, plan.Start)
}

// Remove stops the unit and deletes its file. A unit that is not there is
// not an error.
func Remove(ctx context.Context, plan Plan, run Runner) error {
	if err := runAll(ctx, run, plan.Stop); err != nil {
		return err
	}
	if err := os.Remove(plan.File); err != nil && !errors.Is(err, fs.ErrNotExist) {
		return err
	}
	return runAll(ctx, run, plan.After)
}

// Installed reports whether the file of the unit is there.
func Installed(plan Plan) bool {
	_, err := os.Lstat(plan.File)
	return err == nil
}
