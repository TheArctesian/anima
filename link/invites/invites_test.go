package invites_test

import (
	"bytes"
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"

	"github.com/TheArctesian/anima/link/devices"
	"github.com/TheArctesian/anima/link/invites"
	"github.com/TheArctesian/anima/link/statedir"
	"github.com/TheArctesian/anima/link/tunnel"
	"github.com/TheArctesian/anima/link/tunnel/testbed"
)

func TestMain(m *testing.M) { testbed.Main(m) }

var noon = time.Date(2026, 11, 14, 12, 0, 0, 0, time.UTC)

// hubAddress is a real hub address: an invite holds one.
func hubAddress(t *testing.T) tunnel.Address {
	t.Helper()
	hub, err := tunnel.StartHub(t.Context(), tunnel.HubConfig{
		Key: tunnel.NewPrivateKey(), PresharedKey: tunnel.NewPresharedKey(), Relay: testbed.StartRelay(t),
		Admit: func(tunnel.DeviceKey) bool { return false },
	})
	if err != nil {
		t.Fatal(err)
	}
	defer hub.Close()
	return hub.Address()
}

func openStore(t *testing.T, path string) (*invites.Store, *statedir.Dir) {
	t.Helper()
	dir, err := statedir.Open(path)
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { dir.Close() })
	store, err := invites.Open(dir)
	if err != nil {
		t.Fatal(err)
	}
	return store, dir
}

// counted is a random source that says how many bytes were taken from it.
type counted struct {
	fill  byte
	taken int
}

func (c *counted) Read(p []byte) (int, error) {
	for i := range p {
		p[i] = c.fill
	}
	c.taken += len(p)
	return len(p), nil
}

func TestLNK003TheCodeIs128BitsFromTheGivenSource(t *testing.T) {
	store, _ := openStore(t, filepath.Join(t.TempDir(), "s"))
	hub := hubAddress(t)
	first, second := &counted{fill: 1}, &counted{fill: 2}

	a, _, err := store.Make(hub, "laptop", noon, first)
	if err != nil {
		t.Fatal(err)
	}
	b, _, _ := store.Make(hub, "phone", noon, second)
	again, _, _ := store.Make(hub, "laptop again", noon, &counted{fill: 1})

	if first.taken != 16 {
		t.Errorf("Make took %d bytes from the source, want 16", first.taken)
	}
	if a.Code != bytes16(1) || a.Reveal() != again.Reveal() || a.Reveal() == b.Reveal() {
		t.Error("the code is not the 16 bytes of the source")
	}
	if a.TemporaryKey().Public() == b.TemporaryKey().Public() || a.TemporaryKey().Public() != again.TemporaryKey().Public() {
		t.Error("the temporary key does not follow the code")
	}
}

func bytes16(fill byte) [16]byte { return [16]byte(bytes.Repeat([]byte{fill}, 16)) }

func TestAnInviteComesBackFromItsText(t *testing.T) {
	store, _ := openStore(t, filepath.Join(t.TempDir(), "s"))
	invite, _, err := store.Make(hubAddress(t), "laptop", noon, &counted{fill: 7})
	if err != nil {
		t.Fatal(err)
	}
	text := invite.Reveal()

	parsed, err := invites.Parse("  " + text + "\n")
	if err != nil || parsed.Code != invite.Code || parsed.Hub != invite.Hub {
		t.Fatalf("Parse(Reveal()) changed the invite: %v", err)
	}
	for _, damaged := range []string{"", "hello", text[:len(text)-9], strings.Replace(text, ".1.", ".2.", 1), text[:20] + "!" + text[21:]} {
		_, err := invites.Parse(damaged)
		if !errors.Is(err, invites.ErrBadInvite) {
			t.Errorf("Parse of a damaged text: err = %v, want ErrBadInvite", err)
		}
		if len(damaged) > 30 && strings.Contains(err.Error(), damaged[16:30]) {
			t.Errorf("the error quotes the invite: %q", err)
		}
	}
}

func TestLNK003AnInviteIsOpenFor15MinutesAndForOneUse(t *testing.T) {
	store, _ := openStore(t, filepath.Join(t.TempDir(), "s"))
	hub := hubAddress(t)
	invite, record, _ := store.Make(hub, "laptop", noon, &counted{fill: 1})
	late, _, _ := store.Make(hub, "phone", noon, &counted{fill: 2})
	key, lateKey := invite.TemporaryKey().Public(), late.TemporaryKey().Public()
	stranger := tunnel.NewPrivateKey().Public()

	if !record.Expires.Equal(noon.Add(15 * time.Minute)) {
		t.Errorf("Expires = %v, want 15 minutes after the invite was made", record.Expires)
	}
	if !store.Admits(key, noon.Add(14*time.Minute+59*time.Second)) || store.Admits(lateKey, noon.Add(15*time.Minute)) {
		t.Error("an invite must be open until 15 minutes, and not at 15 minutes")
	}
	if store.Admits(stranger, noon) {
		t.Error("a key with no invite is admitted")
	}

	used, err := store.Use(key, noon.Add(time.Minute))
	if err != nil || used.Name != "laptop" {
		t.Fatalf("first Use = %+v, %v", used, err)
	}
	if _, err := store.Use(key, noon.Add(time.Minute)); !errors.Is(err, invites.ErrNotOpen) {
		t.Errorf("second Use = %v, want ErrNotOpen", err)
	}
	if store.Admits(key, noon.Add(time.Minute)) {
		t.Error("a used invite still admits its key")
	}
	if _, err := store.Use(lateKey, noon.Add(16*time.Minute)); !errors.Is(err, invites.ErrNotOpen) {
		t.Errorf("Use of an expired invite = %v, want ErrNotOpen", err)
	}
}

func TestLNK003AUsedInviteStaysUsedAndAnOpenOneStaysOpenAfterARestart(t *testing.T) {
	path := filepath.Join(t.TempDir(), "s")
	store, dir := openStore(t, path)
	hub := hubAddress(t)
	used, _, _ := store.Make(hub, "laptop", noon, &counted{fill: 1})
	open, _, _ := store.Make(hub, "phone", noon, &counted{fill: 2})
	if _, err := store.Use(used.TemporaryKey().Public(), noon); err != nil {
		t.Fatal(err)
	}
	dir.Close()

	again, _ := openStore(t, path)
	if again.Admits(used.TemporaryKey().Public(), noon) {
		t.Error("a used invite is open again after a restart")
	}
	if !again.Admits(open.TemporaryKey().Public(), noon) {
		t.Error("an open invite is gone after a restart")
	}
	if all := again.All(noon); len(all) != 1 || all[0].Name != "phone" {
		t.Errorf("All = %+v, want the one open invite", all)
	}
	if all := again.All(noon.Add(time.Hour)); len(all) != 0 {
		t.Errorf("All an hour later = %+v, want none", all)
	}
}

func TestLNK003TheHubKeepsNoCodeAndAnInviteDoesNotPrint(t *testing.T) {
	path := filepath.Join(t.TempDir(), "s")
	store, _ := openStore(t, path)
	invite, record, err := store.Make(hubAddress(t), "laptop", noon, &counted{fill: 9})
	if err != nil {
		t.Fatal(err)
	}
	text := invite.Reveal()
	codeText := strings.Split(text, ".")[2]

	file, err := os.ReadFile(filepath.Join(path, "invites.json"))
	if err != nil {
		t.Fatal(err)
	}
	asJSON, _ := json.Marshal(struct {
		Invite invites.Invite
		Record invites.Record
	}{invite, record})
	shown := string(file) + string(asJSON) + fmt.Sprintf("%v %+v %#v %s %x", invite, invite, &invite, invite, invite)
	for name, secret := range map[string]string{
		"code": codeText, "temporary key": invite.TemporaryKey().Reveal(), "hub address": invite.Hub.Reveal(),
		"code bytes": "0909090909",
	} {
		if strings.Contains(shown, secret) {
			t.Errorf("the %s is in the state file or in printed text", name)
		}
	}
}

func TestAnInviteNeedsAValidDeviceName(t *testing.T) {
	store, _ := openStore(t, filepath.Join(t.TempDir(), "s"))
	if _, _, err := store.Make(hubAddress(t), "bad\x1bname", noon, &counted{}); !errors.Is(err, devices.ErrBadName) {
		t.Errorf("err = %v, want ErrBadName", err)
	}
	if len(store.All(noon)) != 0 {
		t.Error("a refused invite is in the list")
	}
}
