package hublink

import (
	"bytes"
	"net/netip"
	"os"
	"testing"
)

// The hub's own tests read the same file (crates/hub, fleet/access/link):
// the two programs agree on these bytes.
func TestTheDeviceHeaderIsTheBytesThatTheHubReads(t *testing.T) {
	want, err := os.ReadFile("testdata/device-header.bin")
	if err != nil {
		t.Fatal(err)
	}
	got := deviceHeader(netip.MustParseAddr("fd41:4e49:4d41:1:2:3:4:5"), 40000, netip.MustParseAddrPort("[fd7a:115c:a1e0::1]:443"))
	if !bytes.Equal(got, want) {
		t.Errorf("header:\n% x\nwant:\n% x", got, want)
	}
}
