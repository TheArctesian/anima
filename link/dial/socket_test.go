package dial_test

import (
	"bufio"
	"context"
	"io"
	"net"
	"net/netip"
	"os"
	"path/filepath"
	"strconv"
	"testing"
	"time"

	"github.com/TheArctesian/anima/link/dial"
)

// dialSocket serves a dial socket with an opener and returns its path.
func dialSocket(t *testing.T, open dial.Opener) string {
	t.Helper()
	// A socket's path is short, so the folder is not under t.TempDir().
	folder, err := os.MkdirTemp("/tmp", "dial")
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { os.RemoveAll(folder) })
	path := filepath.Join(folder, "dial.sock")
	listener, err := net.Listen("unix", path)
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { listener.Close() })
	go dial.Serve(listener, open)
	return path
}

// request writes one request line and returns the answer line and the
// connection after it.
func request(t *testing.T, socket, line string) (string, *bufio.Reader, net.Conn) {
	t.Helper()
	conn, err := net.Dial("unix", socket)
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { conn.Close() })
	_ = conn.SetDeadline(time.Now().Add(10 * time.Second))
	if _, err := io.WriteString(conn, line); err != nil {
		t.Fatal(err)
	}
	reader := bufio.NewReader(conn)
	word, err := reader.ReadString('\n')
	if err != nil {
		t.Fatalf("the dial socket gave no answer line: %v", err)
	}
	return word, reader, conn
}

func TestHMD048TheDialSocketGivesTheConnectionAfterTheLineOK(t *testing.T) {
	port, _ := echo(t)
	b := newBench(t, func(asked uint16) bool { return int(asked) == port })
	released := make(chan struct{})
	socket := dialSocket(t, func(ctx context.Context, device netip.Addr, asked uint16) (net.Conn, func(), error) {
		stream, release, err := b.desk.Open(ctx, device, asked, b.ask)
		return stream, func() { release(); close(released) }, err
	})

	word, reader, conn := request(t, socket, "dial "+theDevice.String()+" "+strconv.Itoa(port)+"\n")
	if word != "ok\n" {
		t.Fatalf("the answer line is %q, want ok", word)
	}
	if _, err := io.WriteString(conn, "through the socket"); err != nil {
		t.Fatal(err)
	}
	got := make([]byte, len("through the socket"))
	if _, err := io.ReadFull(reader, got); err != nil || string(got) != "through the socket" {
		t.Fatalf("the local port answered %q, %v", got, err)
	}
	conn.Close()
	select {
	case <-released:
	case <-time.After(5 * time.Second):
		t.Fatal("the stream was not released 5 s after the hub closed its connection")
	}
}

func TestHMD048TheDialSocketSaysWhyThereIsNoConnection(t *testing.T) {
	failing := func(err error) dial.Opener {
		return func(context.Context, netip.Addr, uint16) (net.Conn, func(), error) { return nil, nil, err }
	}
	for _, c := range []struct {
		err  error
		want string
	}{
		{dial.ErrNotConnected, "not_connected\n"}, {dial.ErrRefused, "refused\n"}, {dial.ErrClosed, "closed\n"},
		{dial.ErrTimeout, "timeout\n"}, {dial.ErrBadRequest, "bad_request\n"}, {io.ErrUnexpectedEOF, "timeout\n"},
	} {
		socket := dialSocket(t, failing(c.err))
		word, reader, _ := request(t, socket, "dial "+theDevice.String()+" 22\n")
		if word != c.want {
			t.Errorf("for %v the socket said %q, want %q", c.err, word, c.want)
		}
		if rest, _ := io.ReadAll(reader); len(rest) != 0 {
			t.Errorf("for %v the socket sent %q after its word", c.err, rest)
		}
	}
}

func TestTheDialSocketRefusesALineThatIsNotARequest(t *testing.T) {
	called := false
	socket := dialSocket(t, func(context.Context, netip.Addr, uint16) (net.Conn, func(), error) {
		called = true
		return nil, nil, dial.ErrClosed
	})
	for _, line := range []string{
		"\n", "dial\n", "dial fd41:4e49:4d41::7\n", "dial not-an-address 22\n", "dial fd41:4e49:4d41::7 0\n",
		"dial fd41:4e49:4d41::7 22 more\n", "open fd41:4e49:4d41::7 22\n", "dial fd41:4e49:4d41::7 70000\n",
	} {
		if word, _, _ := request(t, socket, line); word != "bad_request\n" {
			t.Errorf("for the line %q the socket said %q", line, word)
		}
	}
	if called {
		t.Fatal("a line that is not a request reached the device")
	}
}
