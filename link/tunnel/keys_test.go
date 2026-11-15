package tunnel_test

import (
	"context"
	"errors"
	"fmt"
	"strings"
	"testing"

	"github.com/TheArctesian/anima/link/tunnel"
)

func TestLNK008SecretsDoNotPrint(t *testing.T) {
	key := tunnel.NewPrivateKey()
	psk := tunnel.NewPresharedKey()
	keyText, _ := key.MarshalText()
	pskText, _ := psk.MarshalText()
	address := tunnel.Address("tcSECRETADDRESS")
	config := tunnel.HubConfig{Key: key, PresharedKey: psk}
	device := tunnel.DeviceConfig{Key: key, Hub: address}

	printed := fmt.Sprintf("%v %+v %#v %s %v %+v %#v %v", config, config, config, address, address, device, device, key.Public())
	for name, secret := range map[string]string{
		"private key": string(keyText), "pre-shared key": string(pskText), "address": string(address),
	} {
		if strings.Contains(printed, secret) {
			t.Errorf("the %s is in the printed text", name)
		}
	}
	publicText, _ := key.Public().MarshalText()
	if strings.Contains(printed, string(publicText)) {
		t.Error("a device key prints as the key and not as its fingerprint")
	}
}

func TestLNK008ADamagedKeyTextIsNotInTheError(t *testing.T) {
	pskText, _ := tunnel.NewPresharedKey().MarshalText()
	damaged := append(pskText[:len(pskText)-2:len(pskText)-2], "!!"...)
	var psk tunnel.PresharedKey
	var key tunnel.PrivateKey
	var device tunnel.DeviceKey
	for _, err := range []error{psk.UnmarshalText(damaged), key.UnmarshalText(damaged), device.UnmarshalText(damaged)} {
		if !errors.Is(err, tunnel.ErrBadKey) {
			t.Fatalf("err = %v, want ErrBadKey", err)
		}
		if strings.Contains(err.Error(), string(damaged[:12])) {
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
	if strings.Contains(err.Error(), "relay-host-name") {
		t.Errorf("the error text names the relay map: %q", err)
	}
}
