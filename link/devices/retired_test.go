package devices_test

import (
	"errors"
	"path/filepath"
	"testing"
	"time"

	"github.com/TheArctesian/anima/link/devices"
	"github.com/TheArctesian/anima/link/statedir"
)

// renewed takes a device through a whole key renewal.
func renewed(t *testing.T, store *devices.Store, old, next devices.Device) {
	t.Helper()
	if _, err := store.Renew(old.Key, next.Key); err != nil {
		t.Fatal(err)
	}
	if _, err := store.Promote(next.Key); err != nil {
		t.Fatal(err)
	}
}

func TestLNK004AKeyThatLeftTheListIsNeverPermittedAgain(t *testing.T) {
	today = day
	t.Cleanup(func() { today = day })
	path := filepath.Join(t.TempDir(), "s")
	store, dir := openStore(t, path)
	first, second, other := newKey(), newKey(), newKey()
	laptop := permit(t, store, first, "laptop")
	permit(t, store, other, "desk")

	today = day.Add(devices.KeyLife - 24*time.Hour)
	renewed(t, store, laptop, devices.Device{Key: second})
	if _, err := store.Revoke(second); err != nil {
		t.Fatal(err)
	}
	dir.Close()
	store, _ = openStore(t, path) // the record is on disk

	// The key that the renewal replaced would get the revoked device's
	// address again; so would the revoked key itself.
	for name, key := range map[string]devices.Device{"replaced": {Key: first}, "revoked": {Key: second}} {
		key.Name = "new laptop"
		if _, err := store.Permit(key); !errors.Is(err, devices.ErrRetired) {
			t.Errorf("the %s key: Permit = %v, want ErrRetired", name, err)
		}
	}
	// Nor can a device in its renewal name such a key as its next key.
	for _, gone := range []devices.Device{{Key: first}, {Key: second}} {
		if _, err := store.Renew(other, gone.Key); !errors.Is(err, devices.ErrRetired) {
			t.Errorf("Renew to a key that left the list = %v, want ErrRetired", err)
		}
	}
	if store.Count() != 1 || store.Permits(first) || store.Permits(second) {
		t.Error("a refused key changed the list")
	}
}

func TestLNK005ANextKeyThatWasReplacedBeforeItsUseIsNotPermittedLater(t *testing.T) {
	today = day.Add(devices.KeyLife - 24*time.Hour)
	t.Cleanup(func() { today = day })
	store, _ := openStore(t, filepath.Join(t.TempDir(), "s"))
	key, dropped, kept := newKey(), newKey(), newKey()
	if _, err := store.Permit(devices.Device{Key: key, Name: "laptop", Permitted: day}); err != nil {
		t.Fatal(err)
	}
	for _, next := range []devices.Device{{Key: dropped}, {Key: kept}} {
		if _, err := store.Renew(key, next.Key); err != nil {
			t.Fatal(err)
		}
	}
	if store.Permits(dropped) {
		t.Error("the next key that was replaced is still permitted")
	}
	if _, err := store.Permit(devices.Device{Key: dropped, Name: "other"}); !errors.Is(err, devices.ErrRetired) {
		t.Errorf("Permit of the replaced next key = %v, want ErrRetired", err)
	}
}

func TestAListWithOneAddressForTwoDevicesIsDamaged(t *testing.T) {
	path := filepath.Join(t.TempDir(), "s")
	store, dir := openStore(t, path)
	laptop := permit(t, store, newKey(), "laptop")
	desk := permit(t, store, newKey(), "desk")
	desk.Address = laptop.Address
	if err := dir.Write("devices.json", []devices.Device{laptop, desk}); err != nil {
		t.Fatal(err)
	}
	dir.Close()

	again, err := statedir.Open(path)
	if err != nil {
		t.Fatal(err)
	}
	defer again.Close()
	if _, err := devices.Open(again, time.Now); !errors.Is(err, statedir.ErrDamaged) {
		t.Errorf("Open = %v, want ErrDamaged: the hub would take the two devices for one", err)
	}
}
