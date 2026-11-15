package main

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strings"
	"testing"
	"time"

	"github.com/TheArctesian/anima/link/tunnel"
)

func TestRunRefusesAMissingOrUnknownCommand(t *testing.T) {
	for _, args := range [][]string{nil, {"serve"}, {"hub"}, {"hub", "-state", "x"}, {"ctl", "-state", "x"}} {
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
	var status struct {
		Role, Relay string
		Devices     int
	}
	if err := json.Unmarshal([]byte(text), &status); code != 0 || err != nil {
		t.Fatalf("ctl status = %q, exit %d", text, code)
	}
	if status.Role != "hub" || status.Relay == "" || status.Devices != 0 {
		t.Errorf("status = %+v, want the hub role, its relay and no device", status)
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
	code := run(context.Background(), []string{"hub", "-state", hub.state, "-target", "127.0.0.1:1"}, stdio{strings.NewReader(""), io.Discard, &stderr})
	if code != 1 || !strings.Contains(stderr.String(), "another anima-link") {
		t.Errorf("second hub link: exit %d, %q", code, stderr.String())
	}
}

func TestLNK002TheOperatorPermitsADeviceAndItReachesTheHub(t *testing.T) {
	hub := startHub(t)
	key := tunnel.NewPrivateKey()
	address := hub.address(t)

	if _, err := get(t, key, address, 4*time.Second); err == nil {
		t.Fatal("a device that is not permitted reached the hub")
	}
	if text, code := hub.permit(t, key, "laptop"); code != 0 {
		t.Fatalf("devices.permit: %q, exit %d", text, code)
	}
	if got, err := get(t, key, address, 30*time.Second); err != nil || got != greeting {
		t.Fatalf("the permitted device got %q, %v", got, err)
	}

	listed, _ := hub.ctl(t, "devices.list")
	keyText, _ := key.Public().MarshalText()
	if !strings.Contains(listed, `"name":"laptop"`) || !strings.Contains(listed, string(keyText)) {
		t.Errorf("devices.list = %s, want the laptop and its key", listed)
	}
	if text, code := hub.permit(t, key, "again"); code != 1 || !strings.Contains(text, "already permitted") {
		t.Errorf("a second permit of the same key: %q, exit %d", text, code)
	}
	keyJSON := fmt.Sprintf("%q", keyText)
	for arguments, want := range map[string]string{
		`{"key":` + keyJSON + `,"nam":"typo"}`:  "not valid",
		`{"key":"nodekey:zz","name":"bad key"}`: "not valid",
		`{"name":"no key"}`:                     "needs a key",
	} {
		if text, code := hub.ctl(t, "devices.permit", arguments); code != 1 || !strings.Contains(text, want) {
			t.Errorf("devices.permit %s: %q, exit %d, want %q", arguments, text, code, want)
		}
	}
}

func TestLNK001TheAddressAndTheDevicesAreTheSameAfterARestart(t *testing.T) {
	hub := startHub(t)
	key := tunnel.NewPrivateKey()
	hub.permit(t, key, "laptop")
	address := hub.address(t)
	hub.stdin.Close()
	<-hub.exited

	again := startHubOn(t, hub.state)
	if again.address(t) != address {
		t.Error("the address changed after a restart")
	}
	if got, err := get(t, key, address, 30*time.Second); err != nil || got != greeting {
		t.Errorf("the device that was permitted before the restart got %q, %v", got, err)
	}
}

func TestLNK008NoSecretIsInTheLogOrInAControlAnswer(t *testing.T) {
	hub := startHub(t)
	key := tunnel.NewPrivateKey()
	address := hub.address(t)
	answers, _ := hub.ctl(t, "status")
	unknown, _ := hub.ctl(t, "no-such-verb")
	permitted, _ := hub.permit(t, key, "laptop")
	listed, _ := hub.ctl(t, "devices.list")
	if _, err := get(t, key, address, 30*time.Second); err != nil {
		t.Fatal(err)
	}
	hub.stdin.Close()
	<-hub.exited

	// The address is the answer of address.show and of nothing else.
	everywhereElse := hub.log.String() + answers + unknown + permitted + listed
	for _, secret := range append(secretsOf(t, hub.state), address, address[len(address)/2:]) {
		if strings.Contains(everywhereElse, secret) {
			t.Errorf("a secret of the hub is in the log or in a control answer")
		}
	}
	keyText, _ := key.Public().MarshalText()
	if strings.Contains(hub.log.String(), string(keyText)) {
		t.Error("the log names a device by its key and not by its fingerprint")
	}
}

func TestTheDeviceProgramGivesALocalPortThatReachesTheHub(t *testing.T) {
	hub := startHub(t)
	state, local := startDevice(t, hub)

	response, err := (&http.Client{Timeout: 30 * time.Second}).Get("http://" + local)
	if err != nil {
		t.Fatalf("through the local port of the device program: %v", err)
	}
	body, _ := io.ReadAll(response.Body)
	response.Body.Close()
	if string(body) != greeting {
		t.Errorf("got %q, want the hub's answer", body)
	}

	deadline := time.Now().Add(30 * time.Second)
	for {
		text, _ := runningHub{state: state}.ctl(t, "status")
		if strings.Contains(text, `"path":"direct"`) {
			break
		}
		if time.Now().After(deadline) {
			t.Fatalf("device status = %s, want a direct path on one machine", text)
		}
		time.Sleep(200 * time.Millisecond)
	}
}

func TestADeviceWithNoHubSaysThatItNeedsAnInvite(t *testing.T) {
	var stderr output
	code := run(context.Background(), []string{"device", "-state", stateDir(t)}, stdio{strings.NewReader(""), io.Discard, &stderr})
	if code != 1 || !strings.Contains(stderr.String(), "needs an invite") {
		t.Errorf("device with no hub: exit %d, %q", code, stderr.String())
	}
}

func TestLNK004TheOperatorRevokesADeviceAndItsLocalPortStopsWorking(t *testing.T) {
	hub := startHub(t)
	_, local := startDevice(t, hub)
	client := &http.Client{Timeout: 4 * time.Second, Transport: &http.Transport{DisableKeepAlives: true}}
	if _, err := client.Get("http://" + local); err != nil {
		t.Fatalf("before the revocation: %v", err)
	}
	listed, _ := hub.ctl(t, "devices.list")
	var devices []struct{ Key, Name string }
	if err := json.Unmarshal([]byte(listed), &devices); err != nil || len(devices) != 1 {
		t.Fatalf("devices.list = %s", listed)
	}

	if text, code := hub.ctl(t, "devices.revoke", fmt.Sprintf(`{"key":%q}`, devices[0].Key)); code != 0 {
		t.Fatalf("devices.revoke: %q, exit %d", text, code)
	}
	if _, err := client.Get("http://" + local); err == nil {
		t.Error("the revoked device still reaches the hub through its local port")
	}
	if listed, _ := hub.ctl(t, "devices.list"); strings.Contains(listed, devices[0].Key) {
		t.Errorf("devices.list still shows the revoked device: %s", listed)
	}
	if text, code := hub.ctl(t, "devices.revoke", fmt.Sprintf(`{"key":%q}`, devices[0].Key)); code != 1 || !strings.Contains(text, "not in the list") {
		t.Errorf("a second revoke: %q, exit %d", text, code)
	}
}

func TestLNK003TheOperatorMakesAnInviteAndADeviceJoinsWithIt(t *testing.T) {
	hub := startHub(t)
	made, code := hub.ctl(t, "invites.make", `{"name":"Maya's laptop"}`)
	var invite struct {
		Invite  string
		Expires time.Time
	}
	if err := json.Unmarshal([]byte(made), &invite); code != 0 || err != nil || invite.Invite == "" {
		t.Fatalf("invites.make: exit %d", code)
	}
	if listed, _ := hub.ctl(t, "invites.list"); !strings.Contains(listed, "Maya's laptop") || strings.Contains(listed, invite.Invite[:40]) {
		t.Errorf("invites.list must show the open invite by name and not its text")
	}

	state := stateDir(t)
	var out, errs output
	joinCode := run(context.Background(), []string{"join", "-state", state}, stdio{strings.NewReader(invite.Invite + "\n"), &out, &errs})
	if joinCode != 0 || !strings.Contains(out.String(), "joined") {
		t.Fatalf("join: exit %d, %q %q", joinCode, out.String(), errs.String())
	}
	if listed, _ := hub.ctl(t, "devices.list"); !strings.Contains(listed, "Maya's laptop") {
		t.Errorf("devices.list = %s, want the joined device", listed)
	}
	if listed, _ := hub.ctl(t, "invites.list"); listed != "[]\n" {
		t.Errorf("invites.list after the use = %q, want none", listed)
	}

	again := run(context.Background(), []string{"join", "-state", stateDir(t)}, stdio{strings.NewReader(invite.Invite), io.Discard, &errs})
	if again != 1 || !strings.Contains(errs.String(), "did not answer") {
		t.Errorf("a second join with the same invite: exit %d, %q", again, errs.String())
	}

	for _, shown := range []string{hub.log.String(), out.String(), errs.String()} {
		if strings.Contains(shown, invite.Invite) || strings.Contains(shown, invite.Invite[15:40]) {
			t.Error("the invite is in a log or in the output of join")
		}
	}
}

func TestLNK003TheOperatorCancelsAnInvite(t *testing.T) {
	hub := startHub(t)
	made, _ := hub.ctl(t, "invites.make", `{"name":"phone"}`)
	var invite struct{ Invite string }
	json.Unmarshal([]byte(made), &invite)
	listed, _ := hub.ctl(t, "invites.list")
	var open []struct{ ID string }
	if err := json.Unmarshal([]byte(listed), &open); err != nil || len(open) != 1 {
		t.Fatalf("invites.list = %s", listed)
	}

	if text, code := hub.ctl(t, "invites.cancel", fmt.Sprintf(`{"id":%q}`, open[0].ID)); code != 0 {
		t.Fatalf("invites.cancel: %q, exit %d", text, code)
	}
	if text, code := hub.ctl(t, "invites.cancel", fmt.Sprintf(`{"id":%q}`, open[0].ID)); code != 1 || !strings.Contains(text, "no open invite") {
		t.Errorf("a second cancel: %q, exit %d", text, code)
	}
	var errs output
	if code := run(context.Background(), []string{"join", "-state", stateDir(t)}, stdio{strings.NewReader(invite.Invite), io.Discard, &errs}); code != 1 {
		t.Errorf("join with a cancelled invite: exit %d", code)
	}
	if text, code := hub.ctl(t, "invites.make", `{"name":""}`); code != 1 || !strings.Contains(text, "name") {
		t.Errorf("an invite with no name: %q, exit %d", text, code)
	}
}

func TestLNK006TheOperatorSeesThePathOfEachDevice(t *testing.T) {
	hub := startHub(t)
	_, local := startDevice(t, hub)
	if _, err := (&http.Client{Timeout: 30 * time.Second}).Get("http://" + local); err != nil {
		t.Fatal(err)
	}
	deadline := time.Now().Add(30 * time.Second)
	for {
		listed, _ := hub.ctl(t, "devices.list")
		if strings.Contains(listed, `"path":"direct"`) {
			return
		}
		if time.Now().After(deadline) {
			t.Fatalf("devices.list = %s, want a direct path for the connected device", listed)
		}
		time.Sleep(200 * time.Millisecond)
	}
}
