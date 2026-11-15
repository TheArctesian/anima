package devices_test

import (
	"errors"
	"path/filepath"
	"strings"
	"testing"
	"time"

	"github.com/TheArctesian/anima/link/devices"
	"github.com/TheArctesian/anima/link/statedir"
	"github.com/TheArctesian/anima/link/tunnel"
)

func openStore(t *testing.T, path string) (*devices.Store, *statedir.Dir) {
	t.Helper()
	dir, err := statedir.Open(path)
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { dir.Close() })
	store, err := devices.Open(dir)
	if err != nil {
		t.Fatal(err)
	}
	return store, dir
}

func newKey() tunnel.DeviceKey { return tunnel.NewPrivateKey().Public() }

var day = time.Date(2026, 11, 14, 9, 0, 0, 0, time.UTC)

func TestLNK002OnlyAPermittedKeyIsPermitted(t *testing.T) {
	store, _ := openStore(t, filepath.Join(t.TempDir(), "s"))
	laptop, stranger := newKey(), newKey()
	if store.Permits(laptop) {
		t.Fatal("an empty list permits a key")
	}
	if err := store.Permit(devices.Device{Key: laptop, Name: "laptop", Permitted: day}); err != nil {
		t.Fatal(err)
	}
	if !store.Permits(laptop) {
		t.Error("the permitted key is refused")
	}
	if store.Permits(stranger) {
		t.Error("a key that is not in the list is permitted")
	}
}

func TestLNK002TheListIsTheSameAfterARestart(t *testing.T) {
	path := filepath.Join(t.TempDir(), "s")
	store, dir := openStore(t, path)
	laptop, phone := newKey(), newKey()
	for _, device := range []devices.Device{
		{Key: phone, Name: "phone", Permitted: day.Add(time.Hour)},
		{Key: laptop, Name: "  laptop ", Permitted: day},
	} {
		if err := store.Permit(device); err != nil {
			t.Fatal(err)
		}
	}
	dir.Close()

	again, _ := openStore(t, path)
	if !again.Permits(laptop) || !again.Permits(phone) {
		t.Fatal("a permitted key is refused after a restart")
	}
	all := again.All()
	if len(all) != 2 || all[0].Name != "laptop" || all[1].Name != "phone" || !all[0].Permitted.Equal(day) {
		t.Errorf("All = %+v, want laptop then phone, oldest first, names trimmed", all)
	}
}

func TestADeviceNeedsAKeyANameAndToBeNew(t *testing.T) {
	store, _ := openStore(t, filepath.Join(t.TempDir(), "s"))
	laptop := newKey()
	if err := store.Permit(devices.Device{Key: laptop, Name: "laptop", Permitted: day}); err != nil {
		t.Fatal(err)
	}
	for want, device := range map[error]devices.Device{
		devices.ErrKnown:   {Key: laptop, Name: "the same key", Permitted: day},
		devices.ErrNoKey:   {Name: "no key", Permitted: day},
		devices.ErrBadName: {Key: newKey(), Name: "   ", Permitted: day},
	} {
		if err := store.Permit(device); !errors.Is(err, want) {
			t.Errorf("Permit(%q) = %v, want %v", device.Name, err, want)
		}
	}
	if err := store.Permit(devices.Device{Key: newKey(), Name: strings.Repeat("n", 65), Permitted: day}); !errors.Is(err, devices.ErrBadName) {
		t.Errorf("a name of 65 characters: err = %v, want ErrBadName", err)
	}
	for _, name := range []string{"esc\x1b[2Jape", "c1\u009bcontrol", "right\u202eto-left", "bad\xffutf8", "tab\there"} {
		if err := store.Permit(devices.Device{Key: newKey(), Name: name, Permitted: day}); !errors.Is(err, devices.ErrBadName) {
			t.Errorf("a name that can disguise itself (%q): err = %v, want ErrBadName", name, err)
		}
	}
	if len(store.All()) != 1 {
		t.Errorf("a refused device changed the list: %+v", store.All())
	}
}
