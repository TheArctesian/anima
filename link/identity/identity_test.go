package identity_test

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"

	"github.com/TheArctesian/anima/link/identity"
	"github.com/TheArctesian/anima/link/statedir"
	"github.com/TheArctesian/anima/link/tunnel"
	"github.com/TheArctesian/anima/link/tunnel/testbed"
)

func TestMain(m *testing.M) { testbed.Main(m) }

func open(t *testing.T, path string) *statedir.Dir {
	t.Helper()
	dir, err := statedir.Open(path)
	if err != nil {
		t.Fatalf("Open: %v", err)
	}
	t.Cleanup(func() { dir.Close() })
	return dir
}

// runOnce is one life of a hub link: load the identity, start, save the
// relay that the start chose, report the address, stop.
func runOnce(t *testing.T, path, relayMapURL string) tunnel.Address {
	t.Helper()
	dir := open(t, path)
	defer dir.Close()
	hub, err := identity.LoadHub(dir)
	if err != nil {
		t.Fatalf("LoadHub: %v", err)
	}
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()
	running, err := tunnel.StartHub(ctx, tunnel.HubConfig{
		Key: hub.Key, PresharedKey: hub.PresharedKey, Relay: hub.Relay, RelayMapURL: relayMapURL,
		Admit: func(tunnel.DeviceKey) bool { return false },
	})
	if err != nil {
		t.Fatalf("StartHub: %v", err)
	}
	defer running.Close()
	hub.Relay = running.Relay()
	if err := hub.Save(dir); err != nil {
		t.Fatalf("Save: %v", err)
	}
	return running.Address()
}

func TestLNK001TheAddressIsTheSameAfterARestartWithAnOwnRelay(t *testing.T) {
	path := filepath.Join(t.TempDir(), "hub")
	dir := open(t, path)
	hub, err := identity.LoadHub(dir)
	if err != nil {
		t.Fatal(err)
	}
	hub.Relay = testbed.StartRelay(t)
	if err := hub.Save(dir); err != nil {
		t.Fatal(err)
	}
	dir.Close()

	first := runOnce(t, path, "")
	second := runOnce(t, path, "")
	if first != second {
		t.Error("the address changed after a restart")
	}
}

func TestLNK001TheRelayOfTheFirstStartIsKeptWhenTheMapChanges(t *testing.T) {
	relayMap := func(relay tunnel.Relay) string {
		region, err := json.Marshal(relay)
		if err != nil {
			t.Fatal(err)
		}
		return fmt.Sprintf(`{"Regions":{"1":%s}}`, region)
	}
	served := relayMap(testbed.StartRelay(t))
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {
		fmt.Fprint(w, served)
	}))
	defer server.Close()
	path := filepath.Join(t.TempDir(), "hub")

	first := runOnce(t, path, server.URL)
	served = relayMap(testbed.StartRelay(t))
	second := runOnce(t, path, server.URL)
	if first != second {
		t.Error("the address changed after a restart, although the first relay was saved")
	}
}

func TestLNK001AHubAndADeviceKeepTheirKeys(t *testing.T) {
	path := filepath.Join(t.TempDir(), "state")
	dir := open(t, path)
	hub, err := identity.LoadHub(dir)
	if err != nil {
		t.Fatal(err)
	}
	device, err := identity.LoadDevice(dir)
	if err != nil {
		t.Fatal(err)
	}
	if hub.Key.IsZero() || hub.PresharedKey.IsZero() || device.Key.IsZero() {
		t.Fatal("a first load made no key")
	}
	if hub.Key.Public() == device.Key.Public() {
		t.Error("the hub and the device have the same key")
	}

	hubAgain, _ := identity.LoadHub(dir)
	deviceAgain, _ := identity.LoadDevice(dir)
	if hubAgain.Key.Public() != hub.Key.Public() || deviceAgain.Key.Public() != device.Key.Public() {
		t.Error("a second load gave a different key")
	}
}

func TestLNK008ADamagedIdentityGivesAnErrorWithoutTheKey(t *testing.T) {
	path := filepath.Join(t.TempDir(), "state")
	dir := open(t, path)
	hub, err := identity.LoadHub(dir)
	if err != nil {
		t.Fatal(err)
	}
	keyText := hub.Key.Reveal()
	damaged := fmt.Sprintf(`{"key":"%sXX","presharedKey":"zz"}`, keyText)
	if err := os.WriteFile(filepath.Join(path, "hub.json"), []byte(damaged), 0o600); err != nil {
		t.Fatal(err)
	}
	_, err = identity.LoadHub(dir)
	if !errors.Is(err, statedir.ErrDamaged) {
		t.Fatalf("err = %v, want ErrDamaged", err)
	}
	if strings.Contains(err.Error(), keyText) || strings.Contains(err.Error(), keyText[len(keyText)-16:]) {
		t.Error("the error holds the private key")
	}
}
