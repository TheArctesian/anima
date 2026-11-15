package main

import (
	"context"
	"io"
	"strings"
	"testing"
	"time"

	"github.com/TheArctesian/anima/link/identity"
	"github.com/TheArctesian/anima/link/statedir"
	"github.com/TheArctesian/anima/link/tunnel/testbed"
)

func savedHub(t *testing.T, state string) identity.Hub {
	t.Helper()
	dir, err := statedir.Open(state)
	if err != nil {
		t.Fatal(err)
	}
	defer dir.Close()
	hub, err := identity.LoadHub(dir)
	if err != nil {
		t.Fatal(err)
	}
	return hub
}

func TestLNK007AHubOnARelayThatRefusesItSaysSoAndStops(t *testing.T) {
	state := stateDir(t)
	dir, err := statedir.Open(state)
	if err != nil {
		t.Fatal(err)
	}
	hub, _ := identity.LoadHub(dir)
	hub.Relay = testbed.StartRefusingRelay(t)
	if err := hub.Save(dir); err != nil {
		t.Fatal(err)
	}
	dir.Close()

	var stderr output
	code := run(context.Background(), []string{"hub", "-state", state, "-target", "127.0.0.1:9"}, stdio{strings.NewReader(""), io.Discard, &stderr})
	if code != 1 || !strings.Contains(stderr.String(), "relay does not carry") {
		t.Errorf("exit %d, %q; want a failed start that names the relay as the cause", code, stderr.String())
	}
}

func TestLNK007TheOperatorChangesTheRelayInTwoSteps(t *testing.T) {
	hub := startHub(t)
	before := relayNow(t, hub)

	if text, code := hub.ctl(t, "relay.switch"); code != 1 || !strings.Contains(text, "relay.announce first") {
		t.Errorf("relay.switch with nothing announced: %q, exit %d", text, code)
	}
	if text, code := hub.ctl(t, "relay.announce", `{"relay":"not a host"}`); code != 1 || !strings.Contains(text, "host name") {
		t.Errorf("relay.announce of a bad name: %q, exit %d", text, code)
	}
	if text, code := hub.ctl(t, "relay.announce", `{"relay":"relay.example.org:8443"}`); code != 0 || !strings.Contains(text, `"next":"relay.example.org"`) {
		t.Fatalf("relay.announce: %q, exit %d", text, code)
	}
	if text, _ := hub.ctl(t, "relay.show"); !strings.Contains(text, `"next":"relay.example.org"`) || !strings.Contains(text, `"relay":"`+before+`"`) {
		t.Errorf("relay.show after the announce = %s; the hub must still be on its relay", text)
	}

	if text, code := hub.ctl(t, "relay.switch"); code != 0 || !strings.Contains(text, `"relay":"relay.example.org"`) {
		t.Fatalf("relay.switch: %q, exit %d", text, code)
	}
	select {
	case <-hub.exited:
	case <-time.After(10 * time.Second):
		t.Fatal("the hub link still runs 10 s after relay.switch; it must stop, to start again on the new relay")
	}
	after := savedHub(t, hub.state)
	if after.Relay.Name() != "relay.example.org" || !after.NextRelay.IsZero() {
		t.Errorf("saved relay = %q, next set: %v; want the announced relay as the relay", after.Relay.Name(), !after.NextRelay.IsZero())
	}
}

// relayNow reads the relay name of a running hub link from its status.
func relayNow(t *testing.T, hub runningHub) string {
	t.Helper()
	text, _ := hub.ctl(t, "relay.show")
	name := strings.TrimPrefix(strings.TrimSpace(text), `{"relay":"`)
	return strings.TrimSuffix(name, `"}`)
}

func TestLNK007AHubThatHasARelayDoesNotTakeAnotherAtStart(t *testing.T) {
	state := stateDir(t)
	seedRelay(t, state)
	var stderr output
	code := run(context.Background(), []string{"hub", "-state", state, "-target", "127.0.0.1:9", "-relay", "other.example.org"}, stdio{strings.NewReader(""), io.Discard, &stderr})
	if code != 1 || !strings.Contains(stderr.String(), "relay.announce") {
		t.Errorf("exit %d, %q; want a refusal that names the two verbs", code, stderr.String())
	}
}
