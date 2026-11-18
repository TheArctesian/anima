// Package presence is what a device and the hub link say on the reverse
// port (HMD-047). A device keeps one stream open to that port while its link
// runs, and writes the line "here" on it at once and then each 15 seconds.
// The hub link records the time of the last line of each device, so that the
// hub can tell which devices are connected.
//
// The hub link opens no stream to a device, so this stream is also where the
// hub will ask a device for a local port (HMD-048).
package presence

import (
	"bufio"
	"net"
	"net/netip"
	"sync"
	"time"
)

const (
	// Here is the line that a device writes.
	Here = "here\n"
	// MaxLine is the longest line of either side, without its newline. A
	// longer line ends the stream.
	MaxLine = 256
	// Silence is how long a stream can be silent before the hub link closes
	// it: three lines of the device did not come.
	Silence = 45 * time.Second
)

// Book is the stream on which each device says that it is connected, and the
// time of its last line. A device is named by its link address, which it
// keeps when it renews its key. The zero value is an empty book.
//
// A device has one stream. A second one replaces the first: the device
// opened it because it lost the first.
type Book struct {
	mu      sync.Mutex
	streams map[netip.Addr]net.Conn
	seen    map[netip.Addr]time.Time
}

// Hold makes a stream the stream of a device and returns the one that it
// replaces, if there is one. The caller closes that one.
func (b *Book) Hold(device netip.Addr, stream net.Conn) (replaced net.Conn) {
	b.mu.Lock()
	defer b.mu.Unlock()
	if b.streams == nil {
		b.streams = map[netip.Addr]net.Conn{}
	}
	replaced = b.streams[device]
	b.streams[device] = stream
	return replaced
}

// Release forgets the stream of a device, unless a newer one replaced it.
// The time of the device's last line stays.
func (b *Book) Release(device netip.Addr, stream net.Conn) {
	b.mu.Lock()
	defer b.mu.Unlock()
	if b.streams[device] == stream {
		delete(b.streams, device)
	}
}

// Saw records the time of a line of a device.
func (b *Book) Saw(device netip.Addr, at time.Time) {
	b.mu.Lock()
	defer b.mu.Unlock()
	if b.seen == nil {
		b.seen = map[netip.Addr]time.Time{}
	}
	b.seen[device] = at
}

// Seen is the time of the last line of a device. It reports false for a
// device that wrote none.
func (b *Book) Seen(device netip.Addr) (time.Time, bool) {
	b.mu.Lock()
	defer b.mu.Unlock()
	at, seen := b.seen[device]
	return at, seen
}

// Read reads the lines of a device and calls here for each line "here". It
// returns when the stream ends, is silent for Silence, or gives a line that
// is not "here": the caller then closes the stream.
func Read(stream net.Conn, here func()) {
	lines := bufio.NewReaderSize(stream, MaxLine+1)
	for {
		_ = stream.SetReadDeadline(time.Now().Add(Silence))
		line, err := lines.ReadSlice('\n')
		if err != nil || string(line) != Here {
			return
		}
		here()
	}
}
