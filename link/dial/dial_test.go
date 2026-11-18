package dial_test

import (
	"context"
	"errors"
	"io"
	"net"
	"net/netip"
	"strings"
	"sync/atomic"
	"testing"
	"time"

	"github.com/TheArctesian/anima/link/dial"
)

var (
	theDevice = netip.MustParseAddr("fd41:4e49:4d41::7")
	another   = netip.MustParseAddr("fd41:4e49:4d41::8")
)

// listen makes a listener on loopback and gives each connection to serve.
func listen(t *testing.T, serve func(net.Conn)) *net.TCPAddr {
	t.Helper()
	listener, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { listener.Close() })
	go func() {
		for {
			conn, err := listener.Accept()
			if err != nil {
				return
			}
			go serve(conn)
		}
	}()
	return listener.Addr().(*net.TCPAddr)
}

// bench is a hub desk and one device, with a loopback listener in place of
// the tunnel: the device's streams to the open port arrive at the desk
// under the link address from.
type bench struct {
	desk dial.Desk
	// isOpen is the list of open ports of the device.
	isOpen func(uint16) bool
	// from is the link address that the hub sees for the device's streams.
	from netip.Addr
	// asked gets each line that the hub wrote to the device.
	asked chan string
	// delivered counts the streams that the desk gave back.
	delivered atomic.Int32
	openPort  *net.TCPAddr
}

func newBench(t *testing.T, isOpen func(uint16) bool) *bench {
	t.Helper()
	return newBenchFrom(t, theDevice, isOpen)
}

// newBenchFrom is a bench whose device answers under another link address
// than the one that the hub asks.
func newBenchFrom(t *testing.T, from netip.Addr, isOpen func(uint16) bool) *bench {
	t.Helper()
	b := &bench{isOpen: isOpen, from: from, asked: make(chan string, 4)}
	b.openPort = listen(t, func(stream net.Conn) {
		b.desk.Deliver(b.from, stream)
		stream.Close()
		b.delivered.Add(1)
	})
	return b
}

// ask is the reverse stream: the device reads the line and answers it.
func (b *bench) ask(line string) error {
	b.asked <- line
	request, valid := dial.ParseRequest(strings.TrimSuffix(line, "\n"))
	if !valid {
		return nil
	}
	local := dial.Local{IsOpen: b.isOpen, Hub: func(ctx context.Context) (net.Conn, error) {
		var dialer net.Dialer
		return dialer.DialContext(ctx, "tcp", b.openPort.String())
	}}
	go local.Give(context.Background(), request)
	return nil
}

func (b *bench) open(port int, limit time.Duration) (net.Conn, func(), error) {
	ctx, cancel := context.WithTimeout(context.Background(), limit)
	defer cancel()
	return b.desk.Open(ctx, theDevice, uint16(port), b.ask)
}

// echo is a local program of the device: it sends back what it reads.
func echo(t *testing.T) (port int, connections *atomic.Int32) {
	t.Helper()
	connections = &atomic.Int32{}
	address := listen(t, func(conn net.Conn) {
		connections.Add(1)
		_, _ = io.Copy(conn, conn)
		conn.Close()
	})
	return address.Port, connections
}

func exchange(t *testing.T, stream net.Conn, text string) {
	t.Helper()
	_ = stream.SetDeadline(time.Now().Add(10 * time.Second))
	if _, err := io.WriteString(stream, text); err != nil {
		t.Fatal(err)
	}
	got := make([]byte, len(text))
	if _, err := io.ReadFull(stream, got); err != nil || string(got) != text {
		t.Fatalf("the local port answered %q, %v; want %q", got, err, text)
	}
}

func TestHMD048TheHubReachesAnOpenLocalPortOfTheDevice(t *testing.T) {
	port, _ := echo(t)
	b := newBench(t, func(asked uint16) bool { return int(asked) == port })

	stream, release, err := b.open(port, 10*time.Second)
	if err != nil {
		t.Fatalf("Open: %v", err)
	}
	exchange(t, stream, "from the hub")
	if b.delivered.Load() != 0 {
		t.Fatal("the desk gave the stream back while the hub still used it")
	}
	stream.Close()
	release()
	deadline := time.Now().Add(5 * time.Second)
	for b.delivered.Load() != 1 {
		if time.Now().After(deadline) {
			t.Fatal("the desk still holds the stream 5 s after its release")
		}
		time.Sleep(10 * time.Millisecond)
	}
}

func TestHMD049APortThatIsNotInTheListIsRefusedAndNotConnected(t *testing.T) {
	port, connections := echo(t)
	b := newBench(t, func(uint16) bool { return false })

	if _, _, err := b.open(port, 10*time.Second); !errors.Is(err, dial.ErrRefused) {
		t.Fatalf("Open: %v, want ErrRefused", err)
	}
	if connections.Load() != 0 {
		t.Fatal("the device connected to a port that is not in its list")
	}
	// A device that was given no list has no open port.
	if _, _, err := newBench(t, nil).open(port, 10*time.Second); !errors.Is(err, dial.ErrRefused) {
		t.Fatalf("a device with no list: %v, want ErrRefused", err)
	}
	if connections.Load() != 0 {
		t.Fatal("a device with no list connected to a local port")
	}
}

func TestHMD048AnOpenPortWithNoProgramIsClosed(t *testing.T) {
	listener, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	port := listener.Addr().(*net.TCPAddr).Port
	listener.Close()
	b := newBench(t, func(uint16) bool { return true })

	if _, _, err := b.open(port, 10*time.Second); !errors.Is(err, dial.ErrClosed) {
		t.Fatalf("Open: %v, want ErrClosed", err)
	}
}

func TestHMD048ADeviceWithNoStreamOrNoAnswerGivesNoConnection(t *testing.T) {
	var desk dial.Desk
	notConnected := func(string) error { return errors.New("no stream") }
	if _, _, err := desk.Open(context.Background(), theDevice, 22, notConnected); !errors.Is(err, dial.ErrNotConnected) {
		t.Fatalf("no stream: %v, want ErrNotConnected", err)
	}
	ctx, cancel := context.WithTimeout(context.Background(), 200*time.Millisecond)
	defer cancel()
	silent := func(string) error { return nil }
	if _, _, err := desk.Open(ctx, theDevice, 22, silent); !errors.Is(err, dial.ErrTimeout) {
		t.Fatalf("no answer: %v, want ErrTimeout", err)
	}
	if _, _, err := desk.Open(context.Background(), theDevice, 0, silent); !errors.Is(err, dial.ErrBadRequest) {
		t.Fatalf("port 0: %v, want ErrBadRequest", err)
	}
}

// A device cannot answer the request that the hub made to another device.
func TestHMD048TheAnswerOfAnotherDeviceIsNotTaken(t *testing.T) {
	port, connections := echo(t)
	b := newBenchFrom(t, another, func(uint16) bool { return true })

	if _, _, err := b.open(port, time.Second); !errors.Is(err, dial.ErrTimeout) {
		t.Fatalf("Open: %v, want ErrTimeout", err)
	}
	if connections.Load() != 1 {
		t.Fatalf("the device made %d local connections, want 1: the test must reach the desk", connections.Load())
	}
}

func TestOnlyAWholeOpenLineIsARequest(t *testing.T) {
	id := strings.Repeat("0a", 16)
	if request, valid := dial.ParseRequest("open 22 " + id); !valid || request.Port != 22 {
		t.Fatalf("a valid line was not read: %+v", request)
	}
	for _, line := range []string{
		"", "here", "open", "open 22", "open 22 " + id + " more", "open 0 " + id, "open 022 " + id,
		"open +22 " + id, "open 65536 " + id, "open 22 " + strings.ToUpper(id), "open 22 " + id[:30], "OPEN 22 " + id,
	} {
		if _, valid := dial.ParseRequest(line); valid {
			t.Errorf("the line %q was read as a request", line)
		}
	}
}
