package presence_test

import (
	"io"
	"net"
	"net/netip"
	"strings"
	"testing"
	"time"

	"github.com/TheArctesian/anima/link/presence"
)

// read gives the lines to Read on one end of a pipe and returns how many
// times Read saw "here" before it returned.
func read(t *testing.T, lines string) int {
	t.Helper()
	device, hub := net.Pipe()
	defer hub.Close()
	go func() {
		io.WriteString(device, lines)
		device.Close()
	}()
	count := 0
	done := make(chan struct{})
	go func() {
		presence.Read(hub, func() { count++ })
		close(done)
	}()
	select {
	case <-done:
	case <-time.After(5 * time.Second):
		t.Fatalf("Read still waits 5 s after the lines %q", lines)
	}
	return count
}

func TestHMD047EachLineHereIsCounted(t *testing.T) {
	if got := read(t, "here\nhere\nhere\n"); got != 3 {
		t.Errorf("three lines: counted %d", got)
	}
	if got := read(t, ""); got != 0 {
		t.Errorf("no line: counted %d", got)
	}
}

func TestHMD047ALineThatIsNotHereEndsTheRead(t *testing.T) {
	for name, lines := range map[string]string{
		"another word":      "open 22\nhere\n",
		"no newline":        "here",
		"a long line":       strings.Repeat("h", presence.MaxLine+44) + "\nhere\n",
		"an empty line":     "\nhere\n",
		"a carriage return": "here\r\nhere\n",
	} {
		if got := read(t, lines); got != 0 {
			t.Errorf("%s: counted %d lines, want the read ended", name, got)
		}
	}
	if got := read(t, "here\nHERE\nhere\n"); got != 1 {
		t.Errorf("a wrong line after a good one: counted %d, want 1", got)
	}
}

func TestHMD047TheBookKeepsOneStreamAndTheLastTimeOfEachDevice(t *testing.T) {
	var book presence.Book
	laptop, desktop := netip.MustParseAddr("fd41:4e49:4d41::7"), netip.MustParseAddr("fd41:4e49:4d41::8")
	first, _ := net.Pipe()
	second, _ := net.Pipe()
	if at, seen := book.Seen(laptop); seen {
		t.Fatalf("an empty book saw the device at %v", at)
	}

	if replaced := book.Hold(laptop, first); replaced != nil {
		t.Error("the first stream of a device replaced a stream")
	}
	if replaced := book.Hold(desktop, second); replaced != nil {
		t.Error("the stream of another device replaced a stream")
	}
	if replaced := book.Hold(laptop, second); replaced != first {
		t.Error("the second stream of a device did not replace its first")
	}
	// The first stream ends after the second took its place.
	book.Release(laptop, first)
	if replaced := book.Hold(laptop, first); replaced != second {
		t.Error("the release of a replaced stream removed the stream that replaced it")
	}

	noon := time.Date(2026, 11, 18, 12, 0, 0, 0, time.UTC)
	book.Saw(laptop, noon)
	book.Saw(laptop, noon.Add(15*time.Second))
	book.Release(laptop, first)
	if at, seen := book.Seen(laptop); !seen || !at.Equal(noon.Add(15*time.Second)) {
		t.Errorf("Seen = %v, %v; want the last time, also after the stream ended", at, seen)
	}
	if _, seen := book.Seen(desktop); seen {
		t.Error("a device that wrote no line was seen")
	}
}
