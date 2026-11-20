// Package testhub stands in for the hub on its socket for the link, in
// tests. It reads the header that names the device, as the hub does, and
// answers each request with the link address that the header gave.
package testhub

import (
	"bufio"
	"encoding/binary"
	"errors"
	"fmt"
	"github.com/TheArctesian/anima/link/testpath"
	"io"
	"net"
	"net/http"
	"net/netip"
	"os"
	"path/filepath"
	"sync/atomic"
	"testing"
	"time"
)

// HoldPath is the path of a request that the hub keeps open, as a session.
// The hub answers "held" and then waits for the other side to close.
const HoldPath = "/hold"

// Hub is a hub on a socket.
type Hub struct {
	// Socket is the path that the hub link gets as its target.
	Socket string
	// Ended gets the link address of each held request when it ends.
	Ended chan netip.Addr

	named atomic.Int64
}

// Named is the number of connections that named a device so far.
func (h *Hub) Named() int { return int(h.named.Load()) }

// Start makes the socket in a new folder that only this user can reach.
func Start(t testing.TB) *Hub {
	t.Helper()
	// A socket's path is short, so the folder is not under t.TempDir().
	folder, err := testpath.MkdirTemp("hub")
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { os.RemoveAll(folder) })
	hub := &Hub{Socket: filepath.Join(folder, "link.sock"), Ended: make(chan netip.Addr, 16)}
	listener, err := net.Listen("unix", hub.Socket)
	if err != nil {
		t.Fatal(err)
	}
	if err := testpath.Chmod(hub.Socket, 0o600); err != nil {
		t.Fatal(err)
	}
	server := &http.Server{Handler: http.HandlerFunc(hub.answer), ReadHeaderTimeout: 10 * time.Second}
	go server.Serve(named{listener, hub})
	t.Cleanup(func() { server.Close() })
	return hub
}

func (h *Hub) answer(w http.ResponseWriter, r *http.Request) {
	device := netip.MustParseAddrPort(r.RemoteAddr).Addr()
	if r.URL.Path != HoldPath {
		fmt.Fprint(w, device)
		return
	}
	conn, buffered, err := w.(http.Hijacker).Hijack()
	if err != nil {
		return
	}
	defer conn.Close()
	buffered.WriteString("HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\nheld")
	buffered.Flush()
	io.Copy(io.Discard, conn)
	h.Ended <- device
}

// named is the socket's listener. Each connection it gives has the device's
// link address as its remote address.
type named struct {
	net.Listener
	hub *Hub
}

func (n named) Accept() (net.Conn, error) {
	for {
		conn, err := n.Listener.Accept()
		if err != nil {
			return nil, err
		}
		conn.SetReadDeadline(time.Now().Add(5 * time.Second))
		reader := bufio.NewReader(conn)
		device, err := ReadHeader(reader)
		if err != nil {
			conn.Close()
			continue
		}
		conn.SetReadDeadline(time.Time{})
		n.hub.named.Add(1)
		return namedConn{conn, reader, net.TCPAddrFromAddrPort(device)}, nil
	}
}

type namedConn struct {
	net.Conn
	reader *bufio.Reader
	device net.Addr
}

func (c namedConn) Read(p []byte) (int, error) { return c.reader.Read(p) }
func (c namedConn) RemoteAddr() net.Addr       { return c.device }

// ErrHeader means that a connection did not start with the header that
// names a device.
var ErrHeader = errors.New("testhub: no header that names a device")

var signature = []byte{0x0D, 0x0A, 0x0D, 0x0A, 0x00, 0x0D, 0x0A, 0x51, 0x55, 0x49, 0x54, 0x0A}

// ReadHeader reads the PROXY header (version 2, TCP over IPv6) at the start
// of a connection and returns its source: the device's link address and the
// device's port of the stream.
func ReadHeader(from io.Reader) (netip.AddrPort, error) {
	start := make([]byte, 16)
	if _, err := io.ReadFull(from, start); err != nil {
		return netip.AddrPort{}, ErrHeader
	}
	if string(start[:12]) != string(signature) || start[12] != 0x21 || start[13] != 0x21 {
		return netip.AddrPort{}, ErrHeader
	}
	rest := make([]byte, binary.BigEndian.Uint16(start[14:16]))
	if _, err := io.ReadFull(from, rest); err != nil || len(rest) != 36 {
		return netip.AddrPort{}, ErrHeader
	}
	source := netip.AddrFrom16([16]byte(rest[:16]))
	return netip.AddrPortFrom(source, binary.BigEndian.Uint16(rest[32:34])), nil
}
