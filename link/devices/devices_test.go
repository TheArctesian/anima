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
	store, err := devices.Open(dir, func() time.Time { return today })
	if err != nil {
		t.Fatal(err)
	}
	return store, dir
}

func newKey() tunnel.DeviceKey { return tunnel.NewPrivateKey().Public() }

var day = time.Date(2026, 11, 14, 9, 0, 0, 0, time.UTC)

// today is the clock of the stores in these tests. A test moves it.
var today = day

func TestLNK002OnlyAPermittedKeyIsPermitted(t *testing.T) {
	store, _ := openStore(t, filepath.Join(t.TempDir(), "s"))
	laptop, stranger := newKey(), newKey()
	if store.Permits(laptop) {
		t.Fatal("an empty list permits a key")
	}
	if _, err := store.Permit(devices.Device{Key: laptop, Name: "laptop", Permitted: day}); err != nil {
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
		if _, err := store.Permit(device); err != nil {
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
	if _, err := store.Permit(devices.Device{Key: laptop, Name: "laptop", Permitted: day}); err != nil {
		t.Fatal(err)
	}
	for want, device := range map[error]devices.Device{
		devices.ErrKnown:   {Key: laptop, Name: "the same key", Permitted: day},
		devices.ErrNoKey:   {Name: "no key", Permitted: day},
		devices.ErrBadName: {Key: newKey(), Name: "   ", Permitted: day},
	} {
		if _, err := store.Permit(device); !errors.Is(err, want) {
			t.Errorf("Permit(%q) = %v, want %v", device.Name, err, want)
		}
	}
	if _, err := store.Permit(devices.Device{Key: newKey(), Name: strings.Repeat("n", 65), Permitted: day}); !errors.Is(err, devices.ErrBadName) {
		t.Errorf("a name of 65 characters: err = %v, want ErrBadName", err)
	}
	for _, name := range []string{"esc\x1b[2Jape", "c1\u009bcontrol", "right\u202eto-left", "bad\xffutf8", "tab\there"} {
		if _, err := store.Permit(devices.Device{Key: newKey(), Name: name, Permitted: day}); !errors.Is(err, devices.ErrBadName) {
			t.Errorf("a name that can disguise itself (%q): err = %v, want ErrBadName", name, err)
		}
	}
	if len(store.All()) != 1 {
		t.Errorf("a refused device changed the list: %+v", store.All())
	}
}

func TestLNK004ARevokedKeyIsRefusedAlsoAfterARestart(t *testing.T) {
	path := filepath.Join(t.TempDir(), "s")
	store, dir := openStore(t, path)
	laptop, phone := newKey(), newKey()
	for name, key := range map[string]tunnel.DeviceKey{"laptop": laptop, "phone": phone} {
		if _, err := store.Permit(devices.Device{Key: key, Name: name, Permitted: day}); err != nil {
			t.Fatal(err)
		}
	}

	if _, err := store.Revoke(laptop); err != nil {
		t.Fatal(err)
	}
	if _, err := store.Revoke(laptop); !errors.Is(err, devices.ErrUnknown) {
		t.Errorf("a second Revoke = %v, want ErrUnknown", err)
	}
	if store.Permits(laptop) || !store.Permits(phone) {
		t.Error("Revoke did not remove the one key and keep the other")
	}
	dir.Close()

	again, _ := openStore(t, path)
	if again.Permits(laptop) || !again.Permits(phone) || again.Count() != 1 {
		t.Error("after a restart the revoked key is back, or the other key is gone")
	}
}

const days = 24 * time.Hour

func TestLNK005ADeviceKeyIsPermittedFor180Days(t *testing.T) {
	today = day
	defer func() { today = day }()
	store, _ := openStore(t, filepath.Join(t.TempDir(), "s"))
	laptop := newKey()
	permitted, err := store.Permit(devices.Device{Key: laptop, Name: "laptop"})
	if err != nil {
		t.Fatal(err)
	}
	if !permitted.Permitted.Equal(day) || !permitted.Expires.Equal(day.Add(180*days)) {
		t.Errorf("permitted %v, expires %v; want today and 180 days later", permitted.Permitted, permitted.Expires)
	}

	today = day.Add(179 * days)
	if !store.Permits(laptop) {
		t.Error("the key is refused on day 179")
	}
	today = day.Add(181 * days)
	if store.Permits(laptop) {
		t.Error("the key is permitted on day 181")
	}
}

func TestLNK005AKeyIsRenewedInItsLast30DaysInTwoSteps(t *testing.T) {
	today = day
	defer func() { today = day }()
	path := filepath.Join(t.TempDir(), "s")
	store, dir := openStore(t, path)
	old, next, other := newKey(), newKey(), newKey()
	for name, key := range map[string]tunnel.DeviceKey{"laptop": old, "phone": other} {
		if _, err := store.Permit(devices.Device{Key: key, Name: name}); err != nil {
			t.Fatal(err)
		}
	}

	today = day.Add(151 * days)
	for range 2 { // the device repeats the step when its first try was cut
		if _, err := store.Renew(old, next); err != nil {
			t.Fatalf("Renew: %v", err)
		}
	}
	if !store.Permits(old) || !store.Permits(next) || store.Count() != 2 {
		t.Fatal("between the two steps both keys of the one device must be permitted")
	}
	dir.Close()

	store, _ = openStore(t, path) // a restart between the steps
	promotion, err := store.Promote(next)
	if err != nil || promotion == nil || promotion.Replaced != old || promotion.Device.Key != next {
		t.Fatalf("Promote = %+v, %v; want the device with its next key, in place of the old key", promotion, err)
	}
	if store.Permits(old) || !store.Permits(next) {
		t.Error("after the second step only the new key must be permitted")
	}
	if all := store.All(); len(all) != 2 || all[0].Name != "phone" || all[1].Key != next || !all[1].Expires.Equal(today.Add(180*days)) {
		t.Errorf("list after the renewal = %+v; want the laptop with its new key and 180 more days", all)
	}
	if again, _ := store.Promote(next); again != nil {
		t.Error("a second Promote of the same key changed the list")
	}
}

func TestLNK004RevokingADeviceInARenewalRemovesBothKeys(t *testing.T) {
	today = day.Add(160 * days)
	defer func() { today = day }()
	store, _ := openStore(t, filepath.Join(t.TempDir(), "s"))
	old, next := newKey(), newKey()
	if _, err := store.Permit(devices.Device{Key: old, Name: "laptop", Permitted: day}); err != nil {
		t.Fatal(err)
	}
	if _, err := store.Renew(old, next); err != nil {
		t.Fatal(err)
	}

	revoked, err := store.Revoke(next) // by either key
	if err != nil || revoked.Key != old || revoked.NextKey != next {
		t.Fatalf("Revoke = %+v, %v; want the device with both keys", revoked, err)
	}
	if store.Permits(old) || store.Permits(next) || store.Count() != 0 {
		t.Error("a key of the revoked device is still permitted")
	}
	if promotion, _ := store.Promote(next); promotion != nil {
		t.Error("the next key of a revoked device became a device key")
	}
}

func TestLNK005AnExpiredKeyCannotBeRenewed(t *testing.T) {
	today = day
	defer func() { today = day }()
	store, _ := openStore(t, filepath.Join(t.TempDir(), "s"))
	old := newKey()
	if _, err := store.Permit(devices.Device{Key: old, Name: "laptop"}); err != nil {
		t.Fatal(err)
	}
	today = day.Add(180 * days)
	if _, err := store.Renew(old, newKey()); !errors.Is(err, devices.ErrUnknown) {
		t.Errorf("a renewal at the expiry time: err = %v, want ErrUnknown (the device needs a new invite)", err)
	}
}

func TestLNK005ARenewalIsRefusedWhenItIsNotDueOrItsKeysAreWrong(t *testing.T) {
	today = day
	defer func() { today = day }()
	store, _ := openStore(t, filepath.Join(t.TempDir(), "s"))
	old, next, other := newKey(), newKey(), newKey()
	for name, key := range map[string]tunnel.DeviceKey{"laptop": old, "phone": other} {
		if _, err := store.Permit(devices.Device{Key: key, Name: name}); err != nil {
			t.Fatal(err)
		}
	}

	today = day.Add(149 * days)
	if _, err := store.Renew(old, next); !errors.Is(err, devices.ErrNotDue) {
		t.Errorf("a renewal 31 days before the expiry: err = %v, want ErrNotDue", err)
	}
	today = day.Add(151 * days)
	for why, try := range map[string]struct {
		current, next tunnel.DeviceKey
		want          error
	}{
		"a next key that another device has":   {old, other, devices.ErrKnown},
		"a next key that is the current key":   {old, old, devices.ErrKnown},
		"no next key":                          {old, tunnel.DeviceKey{}, devices.ErrNoKey},
		"a renewal by a key that is no device": {next, newKey(), devices.ErrUnknown},
	} {
		if _, err := store.Renew(try.current, try.next); !errors.Is(err, try.want) {
			t.Errorf("%s: err = %v, want %v", why, err, try.want)
		}
	}
	if store.Permits(next) {
		t.Error("a refused renewal permitted a key")
	}
}
