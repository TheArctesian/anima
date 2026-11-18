package dial

import (
	"context"
	"io"
	"net"
	"net/netip"
	"time"

	"github.com/TheArctesian/anima/link/tunnel"
)

// Opener gives the connection to a local port of a device, as Desk.Open
// does.
type Opener func(ctx context.Context, device netip.Addr, port uint16) (stream net.Conn, release func(), err error)

// Serve answers the dial socket of the hub link until the listener ends.
// Each connection is one request: see the package comment.
func Serve(listener net.Listener, open Opener) {
	for {
		local, err := listener.Accept()
		if err != nil {
			return
		}
		go serve(local, open)
	}
}

func serve(local net.Conn, open Opener) {
	_ = local.SetDeadline(time.Now().Add(lineLimit))
	line, read := readLine(local)
	device, port, valid := parseDial(line)
	if !read || !valid {
		say(local, WordBadRequest)
		local.Close()
		return
	}
	ctx, cancel := context.WithTimeout(context.Background(), AnswerLimit)
	stream, release, err := open(ctx, device, port)
	cancel()
	if err != nil {
		say(local, Word(err))
		local.Close()
		return
	}
	defer release()
	if !say(local, WordOK) {
		local.Close()
		stream.Close()
		return
	}
	_ = local.SetDeadline(time.Time{})
	tunnel.Join(local, stream)
}

// say writes the answer line of the dial socket.
func say(local net.Conn, word string) bool {
	_ = local.SetWriteDeadline(time.Now().Add(lineLimit))
	_, err := io.WriteString(local, word+"\n")
	return err == nil
}
