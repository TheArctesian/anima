// Package testbed gives tests a relay on loopback, so that no test of the
// link needs the internet.
package testbed

import (
	"context"
	"encoding/json"
	"os"
	"testing"

	"github.com/TheArctesian/anima/link/tunnel"
	"tailscale.com/envknob"
	"tailscale.com/net/netcheck"
	"tailscale.com/syncs"
	"tailscale.com/tailcfg"
	"tailscale.com/tstest/integration"
	"tailscale.com/types/logger"
)

// Main is the TestMain of each package that starts a tunnel. It stops the
// library's checks of the outside network, then runs the tests.
func Main(m *testing.M) {
	envknob.Setenv("IN_TS_TEST", "true")
	netcheck.HookStartCaptivePortalDetection.SetForTest(func(context.Context, *netcheck.Client, *tailcfg.DERPMap, tailcfg.DERPRegionID, func(bool)) (<-chan struct{}, func()) {
		return syncs.ClosedChan(), func() {}
	})
	os.Exit(m.Run())
}

// StartRelay starts a relay on loopback for one test.
func StartRelay(t testing.TB) tunnel.Relay {
	t.Helper()
	regions := integration.RunDERPAndSTUN(t, logger.Discard, "127.0.0.1").Regions
	text, err := json.Marshal(regions[1])
	if err != nil {
		t.Fatal(err)
	}
	var relay tunnel.Relay
	if err := json.Unmarshal(text, &relay); err != nil {
		t.Fatal(err)
	}
	return relay
}

// ForceRelay stops direct paths for one test, so that all traffic uses the
// relay. Call it before the test starts a hub or a device. Tests that use it
// must not run in parallel with other tunnel tests.
func ForceRelay(t testing.TB) {
	t.Helper()
	envknob.Setenv("TS_DEBUG_ALWAYS_USE_DERP", "true")
	t.Cleanup(func() { envknob.Setenv("TS_DEBUG_ALWAYS_USE_DERP", "") })
}
