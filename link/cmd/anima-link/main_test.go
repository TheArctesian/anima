package main

import (
	"bytes"
	"context"
	"encoding/json"
	"io"
	"os"
	"path/filepath"
	"strings"
	"sync"
	"testing"
	"time"
)

// output is a buffer that the command and the test can use at the same time.
type output struct {
	mu   sync.Mutex
	text bytes.Buffer
}

func (o *output) Write(p []byte) (int, error) {
	o.mu.Lock()
	defer o.mu.Unlock()
	return o.text.Write(p)
}

func (o *output) String() string {
	o.mu.Lock()
	defer o.mu.Unlock()
	return o.text.String()
}

// stateDir is a short path: a Unix socket path has a limit of about 100 bytes,
// and the test's own temporary directory is too long on macOS.
func stateDir(t *testing.T) string {
	t.Helper()
	path, err := os.MkdirTemp("/tmp", "anima-link-")
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { os.RemoveAll(path) })
	return filepath.Join(path, "s")
}

// runningHub is a hub link that a test started.
type runningHub struct {
	state  string
	stdin  io.Closer
	log    *output
	exited chan int
}

func startHub(t *testing.T) runningHub {
	t.Helper()
	state := stateDir(t)
	stdinReader, stdinWriter := io.Pipe()
	hub := runningHub{state: state, stdin: stdinWriter, log: &output{}, exited: make(chan int, 1)}
	go func() {
		hub.exited <- run(context.Background(), []string{"hub", "-state", state, "-stop-with-stdin"}, stdio{stdinReader, io.Discard, hub.log})
	}()
	t.Cleanup(func() { stdinWriter.Close() })
	deadline := time.Now().Add(20 * time.Second)
	for !strings.Contains(hub.log.String(), "ready") {
		if time.Now().After(deadline) {
			t.Fatalf("the hub link did not start: %s", hub.log)
		}
		time.Sleep(10 * time.Millisecond)
	}
	return hub
}

// ctl runs one control verb as a second process would.
func (h runningHub) ctl(t *testing.T, args ...string) (stdout string, code int) {
	t.Helper()
	var out, errs output
	code = run(context.Background(), append([]string{"ctl", "-state", h.state}, args...), stdio{strings.NewReader(""), &out, &errs})
	return out.String() + errs.String(), code
}

func TestRunRefusesAMissingOrUnknownCommand(t *testing.T) {
	for _, args := range [][]string{nil, {"serve"}, {"hub"}, {"ctl", "-state", "x"}} {
		var stderr output
		if code := run(context.Background(), args, stdio{strings.NewReader(""), io.Discard, &stderr}); code != 2 {
			t.Errorf("run(%v) = %d, want 2", args, code)
		}
		if !strings.Contains(stderr.String(), "usage: anima-link") {
			t.Errorf("run(%v) did not print the usage: %q", args, stderr.String())
		}
	}
}

func TestASecondProcessAsksTheRunningHubLinkForItsStatus(t *testing.T) {
	hub := startHub(t)

	text, code := hub.ctl(t, "status")
	var status struct{ Role, Key string }
	if err := json.Unmarshal([]byte(text), &status); code != 0 || err != nil {
		t.Fatalf("ctl status = %q, exit %d", text, code)
	}
	if status.Role != "hub" || len(status.Key) != 8 {
		t.Errorf("status = %+v, want the hub role and a key fingerprint", status)
	}

	if text, code := hub.ctl(t, "no-such-verb"); code != 1 || !strings.Contains(text, "no such verb") {
		t.Errorf("unknown verb: %q, exit %d", text, code)
	}
}

func TestCtlSaysWhenNoLinkRuns(t *testing.T) {
	text, code := runningHub{state: stateDir(t)}.ctl(t, "status")
	if code != 1 || !strings.Contains(text, "no anima-link runs") {
		t.Errorf("ctl with no hub link: %q, exit %d", text, code)
	}
}

func TestTheHubLinkStopsWhenItsParentGoesAway(t *testing.T) {
	hub := startHub(t)
	hub.stdin.Close()
	select {
	case code := <-hub.exited:
		if code != 0 {
			t.Errorf("exit code = %d, want 0", code)
		}
	case <-time.After(2 * time.Second):
		t.Fatal("the hub link still runs 2 s after its standard input closed")
	}
	if _, code := hub.ctl(t, "status"); code != 1 {
		t.Error("the control socket still answers after the stop")
	}
}

func TestASecondHubLinkOnTheSameStateDirectoryIsRefused(t *testing.T) {
	hub := startHub(t)
	var stderr output
	code := run(context.Background(), []string{"hub", "-state", hub.state}, stdio{strings.NewReader(""), io.Discard, &stderr})
	if code != 1 || !strings.Contains(stderr.String(), "another anima-link") {
		t.Errorf("second hub link: exit %d, %q", code, stderr.String())
	}
}

// secretsOf reads every secret that a state directory holds, as the texts
// that would show in a log.
func secretsOf(t *testing.T, state string) []string {
	t.Helper()
	text, err := os.ReadFile(filepath.Join(state, "hub.json"))
	if err != nil {
		t.Fatal(err)
	}
	var saved struct{ Key, PresharedKey string }
	if err := json.Unmarshal(text, &saved); err != nil {
		t.Fatal(err)
	}
	if saved.Key == "" || saved.PresharedKey == "" {
		t.Fatal("the state file holds no key: the log test would prove nothing")
	}
	return []string{saved.Key, strings.TrimPrefix(saved.Key, "privkey:"), saved.PresharedKey}
}

func TestLNK008NoSecretIsInTheLogOrInAControlAnswer(t *testing.T) {
	hub := startHub(t)
	answers, _ := hub.ctl(t, "status")
	unknown, _ := hub.ctl(t, "no-such-verb")
	hub.stdin.Close()
	<-hub.exited

	shown := hub.log.String() + answers + unknown
	for _, secret := range secretsOf(t, hub.state) {
		if strings.Contains(shown, secret) {
			t.Errorf("a secret of the state directory is in the output of the run")
		}
	}
}
