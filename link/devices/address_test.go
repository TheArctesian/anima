package devices_test

import (
	"errors"
	"net/netip"
	"path/filepath"
	"testing"
	"time"

	"github.com/TheArctesian/anima/link/devices"
	"github.com/TheArctesian/anima/link/tunnel"
)

func permit(t *testing.T, store *devices.Store, key tunnel.DeviceKey, name string) devices.Device {
	t.Helper()
	device, err := store.Permit(devices.Device{Key: key, Name: name, Permitted: day})
	if err != nil {
		t.Fatal(err)
	}
	return device
}

func TestEachPermittedDeviceHasItsOwnAddressInTheLinkRange(t *testing.T) {
	store, _ := openStore(t, filepath.Join(t.TempDir(), "s"))
	laptop, desk, stranger := newKey(), newKey(), newKey()
	permit(t, store, laptop, "laptop")
	permit(t, store, desk, "desk")

	first, known := store.AddressOf(laptop)
	if !known || !devices.LinkRange().Contains(first) {
		t.Fatalf("the laptop's address is %v (known %v), not in %v", first, known, devices.LinkRange())
	}
	second, _ := store.AddressOf(desk)
	if first == second {
		t.Error("two devices have one address")
	}
	if address, known := store.AddressOf(stranger); known || address.IsValid() {
		t.Errorf("a key that is not in the list has the address %v", address)
	}
}

func TestTheAddressOfAKeyDoesNotChangeBetweenVersions(t *testing.T) {
	store, _ := openStore(t, filepath.Join(t.TempDir(), "s"))
	key := tunnel.PrivateKeyFromSeed([32]byte{1}).Public()
	permit(t, store, key, "laptop")
	address, _ := store.AddressOf(key)
	// A hub binds builders and sessions to this address. A change of the
	// rule would give each new device of an old hub a different kind of
	// address; the addresses in a saved list do not change.
	if want := netip.MustParseAddr(pinnedAddress); address != want {
		t.Errorf("the address is %v, want %v", address, want)
	}
}

func TestADeviceKeepsItsAddressAfterARestart(t *testing.T) {
	path := filepath.Join(t.TempDir(), "s")
	store, dir := openStore(t, path)
	laptop := newKey()
	permit(t, store, laptop, "laptop")
	before, _ := store.AddressOf(laptop)
	dir.Close()

	again, _ := openStore(t, path)
	if after, known := again.AddressOf(laptop); !known || after != before {
		t.Errorf("after a restart the address is %v (known %v), want %v", after, known, before)
	}
}

func TestLNK005ADeviceKeepsItsAddressWhenItRenewsItsKey(t *testing.T) {
	today = day
	t.Cleanup(func() { today = day })
	store, _ := openStore(t, filepath.Join(t.TempDir(), "s"))
	old, next := newKey(), newKey()
	permit(t, store, old, "laptop")
	address, _ := store.AddressOf(old)

	today = day.Add(devices.KeyLife - 24*time.Hour)
	if _, err := store.Renew(old, next); err != nil {
		t.Fatal(err)
	}
	if during, known := store.AddressOf(next); !known || during != address {
		t.Errorf("in a renewal the next key has the address %v (known %v), want %v", during, known, address)
	}
	if _, err := store.Promote(next); err != nil {
		t.Fatal(err)
	}
	if after, known := store.AddressOf(next); !known || after != address {
		t.Errorf("after a renewal the address is %v (known %v), want %v", after, known, address)
	}
	if _, known := store.AddressOf(old); known {
		t.Error("the replaced key still has an address")
	}
}

func TestLNK004ARevokedOrExpiredKeyHasNoAddress(t *testing.T) {
	today = day
	t.Cleanup(func() { today = day })
	store, _ := openStore(t, filepath.Join(t.TempDir(), "s"))
	revoked, expired := newKey(), newKey()
	permit(t, store, revoked, "laptop")
	permit(t, store, expired, "desk")

	if _, err := store.Revoke(revoked); err != nil {
		t.Fatal(err)
	}
	if _, known := store.AddressOf(revoked); known {
		t.Error("a revoked key has an address")
	}
	today = day.Add(devices.KeyLife)
	if _, known := store.AddressOf(expired); known {
		t.Error("an expired key has an address (LNK-005)")
	}
}

func TestAListFromBeforeAddressesGivesEachDeviceItsAddress(t *testing.T) {
	path := filepath.Join(t.TempDir(), "s")
	_, dir := openStore(t, path)
	laptop := newKey()
	old := []map[string]any{{"key": laptop, "name": "laptop", "permitted": day, "expires": day.Add(devices.KeyLife)}}
	if err := dir.Write("devices.json", old); err != nil {
		t.Fatal(err)
	}
	dir.Close()

	store, _ := openStore(t, path)
	if address, known := store.AddressOf(laptop); !known || !devices.LinkRange().Contains(address) {
		t.Errorf("the address is %v (known %v)", address, known)
	}
}

func TestAKeyWhoseAddressIsTakenIsRefused(t *testing.T) {
	store, _ := openStore(t, filepath.Join(t.TempDir(), "s"))
	if !errors.Is(devices.PermitBesideItsOwnAddress(store, newKey()), devices.ErrAddressTaken) {
		t.Error("a key was permitted with the address of another device")
	}
}

const pinnedAddress = "fd41:4e49:4d41:5acc:e129:e384:e0d6:a2a1"
