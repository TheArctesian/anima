package tunnel_test

import (
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"log/slog"
	"strings"
	"testing"

	"github.com/TheArctesian/anima/link/tunnel"
)

// everyPrinting gives the text of each ordinary way to print or log values.
func everyPrinting(t *testing.T, values ...any) string {
	t.Helper()
	var shown bytes.Buffer
	for _, verb := range []string{"%v", "%+v", "%#v", "%s", "%q", "%d", "%x", "%X"} {
		for _, value := range values {
			fmt.Fprintf(&shown, verb+"\n", value)
		}
	}
	for _, logger := range []*slog.Logger{slog.New(slog.NewTextHandler(&shown, nil)), slog.New(slog.NewJSONHandler(&shown, nil))} {
		for _, value := range values {
			logger.Info("value", "is", value)
		}
	}
	for _, value := range values {
		text, err := json.Marshal(value)
		if err != nil {
			t.Fatalf("json.Marshal: %v", err)
		}
		shown.Write(text)
	}
	return shown.String()
}

func TestLNK008NoWayOfPrintingShowsASecret(t *testing.T) {
	hub := startHub(t, newIdentity(t), admitAll)
	key, psk, address := tunnel.NewPrivateKey(), tunnel.NewPresharedKey(), hub.Address()
	hubConfig := tunnel.HubConfig{Key: key, PresharedKey: psk}
	deviceConfig := tunnel.DeviceConfig{Key: key, Hub: address}

	saved := struct {
		Key     tunnel.PrivateKey
		PSK     *tunnel.PresharedKey
		Address tunnel.Address
	}{key, &psk, address}
	shown := strings.ToLower(everyPrinting(t, key, psk, address, &key, saved) +
		fmt.Sprintf("%v %+v %#v %v %+v %#v", hubConfig, hubConfig, hubConfig, deviceConfig, deviceConfig, deviceConfig))

	secrets := map[string]string{"private key": key.Reveal(), "pre-shared key": psk.Reveal(), "address": address.Reveal()}
	for name, text := range secrets {
		body := text[strings.Index(text, ":")+1:]
		if len(body) < 40 {
			t.Fatalf("the %s text is too short to prove anything", name)
		}
		if strings.Contains(shown, strings.ToLower(body[:24])) {
			t.Errorf("the %s is in the printed text", name)
		}
	}
	if !strings.Contains(shown, "[secret]") {
		t.Error("nothing printed as [secret]: the test printed no secret value")
	}
}

func TestLNK008ADeviceKeyPrintsAndLogsAsItsFingerprint(t *testing.T) {
	device := tunnel.NewPrivateKey().Public()
	keyText, _ := device.MarshalText()
	var shown bytes.Buffer
	fmt.Fprintf(&shown, "%v %s\n", device, device)
	slog.New(slog.NewTextHandler(&shown, nil)).Info("device", "key", device)
	slog.New(slog.NewJSONHandler(&shown, nil)).Info("device", "key", device)

	if strings.Contains(shown.String(), string(keyText)) {
		t.Error("a device key shows as the key")
	}
	if strings.Count(shown.String(), device.Fingerprint()) != 4 {
		t.Errorf("want the fingerprint four times, got %q", shown.String())
	}
}

func TestASecretComesBackFromItsRevealedText(t *testing.T) {
	hub := startHub(t, newIdentity(t), admitAll)
	key, psk := tunnel.NewPrivateKey(), tunnel.NewPresharedKey()

	keyAgain, err := tunnel.ParsePrivateKey(key.Reveal())
	if err != nil || keyAgain.Public() != key.Public() {
		t.Errorf("private key: %v", err)
	}
	pskAgain, err := tunnel.ParsePresharedKey(psk.Reveal())
	if err != nil || pskAgain.Reveal() != psk.Reveal() {
		t.Errorf("pre-shared key: %v", err)
	}
	addressAgain, err := tunnel.ParseAddress(hub.Address().Reveal())
	if err != nil || addressAgain != hub.Address() {
		t.Errorf("address: %v", err)
	}
}

func TestLNK008ADamagedSecretTextIsNotInTheError(t *testing.T) {
	text := tunnel.NewPresharedKey().Reveal()
	damaged := text[:len(text)-2] + "!!"
	_, pskErr := tunnel.ParsePresharedKey(damaged)
	_, keyErr := tunnel.ParsePrivateKey(damaged)
	_, addressErr := tunnel.ParseAddress(damaged)
	var device tunnel.DeviceKey
	for _, err := range []error{pskErr, keyErr, addressErr, device.UnmarshalText([]byte(damaged))} {
		if !errors.Is(err, tunnel.ErrBadKey) && !errors.Is(err, tunnel.ErrBadAddress) {
			t.Fatalf("err = %v, want ErrBadKey or ErrBadAddress", err)
		}
		if strings.Contains(err.Error(), damaged[:12]) {
			t.Errorf("the error quotes the text: %q", err)
		}
	}
}

func TestLNK008AStartErrorKeepsItsCauseOutOfItsText(t *testing.T) {
	ctx, cancel := context.WithCancel(context.Background())
	cancel()
	_, err := tunnel.StartHub(ctx, tunnel.HubConfig{
		Key: tunnel.NewPrivateKey(), PresharedKey: tunnel.NewPresharedKey(),
		RelayMapURL: "http://127.0.0.1:1/relay-host-name.json", Admit: admitAll,
	})
	if !errors.Is(err, tunnel.ErrStart) {
		t.Fatalf("err = %v, want ErrStart", err)
	}
	if !errors.Is(err, context.Canceled) {
		t.Errorf("the cause is not reachable: errors.Is(err, context.Canceled) is false for %#v", err)
	}
	if strings.Contains(err.Error(), "relay-host-name") {
		t.Errorf("the error text names the relay map: %q", err)
	}
}
