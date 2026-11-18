package devicelink

import (
	"bufio"
	"context"
	"io"
	"net"
	"strings"
	"time"

	"github.com/TheArctesian/anima/link/dial"
	"github.com/TheArctesian/anima/link/presence"
	"github.com/TheArctesian/anima/link/tunnel"
)

// The device keeps one stream open to the hub's reverse port while the link
// runs (HMD-047). On it the device writes the line "here" at once and then
// at each keepalive, so that the hub knows that the device is connected.
// The hub cannot open a stream to a device, so this stream is also where
// the hub asks the device for a local port (HMD-048): the device answers
// each request on a new stream to the open port of the hub (package dial).
//
// A hub that is older than the reverse port refuses the stream. The device
// then tries again with longer waits and says nothing: the local port and
// the keepalive do not use this stream.

const (
	// The wait before the device opens the stream again doubles from the
	// first value to the limit. A stream that stayed open starts it again.
	presenceRetryFirst = 5 * time.Second
	presenceRetryLimit = 60 * time.Second
	// maxLocalConnections is how many connections to local ports the hub
	// can have at one time. A request above it gets no answer.
	maxLocalConnections = 64
)

// stayPresent keeps the stream to the reverse port open until the link
// stops.
func (l *Link) stayPresent(ctx context.Context, every time.Duration) {
	defer l.done.Done()
	wait := presenceRetryFirst
	for {
		if l.present(ctx, every) {
			wait = presenceRetryFirst
		}
		select {
		case <-ctx.Done():
			return
		case <-l.presentNow:
			wait = presenceRetryFirst
			continue
		case <-time.After(wait):
		}
		wait = min(2*wait, presenceRetryLimit)
	}
}

// present opens one stream to the reverse port and writes "here" on it until
// the stream ends. It reports whether the stream stayed open for one
// keepalive or more: the hub then has the reverse port.
func (l *Link) present(ctx context.Context, every time.Duration) (held bool) {
	dialCtx, cancel := context.WithTimeout(ctx, dialLimit)
	stream, err := l.tunnel().Dial(dialCtx, tunnel.ReversePort)
	cancel()
	if err != nil {
		return false
	}
	l.setReverse(stream)
	ended := make(chan struct{})
	go func() {
		defer close(ended)
		l.readRequests(ctx, stream)
	}()
	defer func() {
		stream.Close()
		<-ended
	}()
	ticker := time.NewTicker(every)
	defer ticker.Stop()
	for {
		_ = stream.SetWriteDeadline(time.Now().Add(l.pingLimit))
		if _, err := io.WriteString(stream, presence.Here); err != nil {
			return held
		}
		select {
		case <-ctx.Done():
			return held
		case <-ended:
			return held
		case <-ticker.C:
			held = true
		}
	}
}

// readRequests reads the lines of the hub until the stream ends. A line that
// asks for a local port (HMD-048) is answered on a stream of its own; any
// other line is dropped.
func (l *Link) readRequests(ctx context.Context, stream net.Conn) {
	lines := bufio.NewReaderSize(stream, presence.MaxLine+1)
	for {
		line, err := lines.ReadSlice('\n')
		if err != nil {
			return
		}
		request, asked := dial.ParseRequest(strings.TrimSuffix(string(line), "\n"))
		if !asked {
			continue
		}
		select {
		case l.giving <- struct{}{}:
			go func() {
				defer func() { <-l.giving }()
				l.local().Give(ctx, request)
			}()
		default:
		}
	}
}

// local is what the device gives the hub of its own ports: the ports of
// Config.OpenPort, and a stream to the open port of the hub for each.
func (l *Link) local() dial.Local {
	return dial.Local{IsOpen: l.openPort, Hub: func(ctx context.Context) (net.Conn, error) {
		ctx, cancel := context.WithTimeout(ctx, dialLimit)
		defer cancel()
		return l.tunnel().Dial(ctx, tunnel.OpenPort)
	}}
}

func (l *Link) setReverse(stream net.Conn) {
	l.mu.Lock()
	defer l.mu.Unlock()
	l.reverse = stream
}

// presentAgain ends the stream to the reverse port and opens a new one with
// no wait. The keepalive calls it when the device had to register again: the
// hub then has forgotten the stream, and the tunnel reports that only after
// its own long limit.
func (l *Link) presentAgain() {
	l.mu.Lock()
	stream := l.reverse
	l.mu.Unlock()
	if stream != nil {
		stream.Close()
	}
	select {
	case l.presentNow <- struct{}{}:
	default:
	}
}
