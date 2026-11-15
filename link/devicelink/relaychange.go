package devicelink

import (
	"bufio"
	"context"
	"encoding/json"

	"github.com/TheArctesian/anima/link/invites"
	"github.com/TheArctesian/anima/link/tunnel"
)

// failuresBeforeNextHub is how many keepalives in a row must fail before
// the device tries the address that the hub announced.
const failuresBeforeNextHub = 3

// readNotice asks the hub for its next address and keeps it (LNK-007).
func (l *Link) readNotice(ctx context.Context) {
	if l.save == nil {
		return
	}
	ctx, cancel := context.WithTimeout(ctx, pingLimit)
	defer cancel()
	stream, err := l.tunnel().Dial(ctx, tunnel.NoticePort)
	if err != nil {
		return
	}
	defer stream.Close()
	if deadline, ok := ctx.Deadline(); ok {
		_ = stream.SetDeadline(deadline)
	}
	var notice invites.Notice
	line, err := bufio.NewReader(stream).ReadBytes('\n')
	if err != nil || json.Unmarshal(line, &notice) != nil || notice.NextHub == "" {
		return
	}
	next, err := tunnel.ParseAddress(notice.NextHub)
	current := l.identity()
	if err != nil || next == current.NextHub || next == current.Hub {
		return
	}
	current.NextHub = next
	if l.save(current) == nil {
		l.setIdentity(current)
	}
}

// followHub tries the address that the hub announced, after the hub has
// stopped answering at its address. When the hub answers there, that
// address becomes the hub's address.
func (l *Link) followHub(ctx context.Context) {
	current := l.identity()
	if l.save == nil || current.NextHub.IsZero() {
		return
	}
	moved, err := tunnel.NewDevice(tunnel.DeviceConfig{Key: current.Key, Hub: current.NextHub, DebugLogf: l.debugLogf})
	if err != nil {
		return
	}
	ctx, cancel := context.WithTimeout(ctx, registerLimit)
	defer cancel()
	if moved.Register(ctx) != nil {
		moved.Close()
		return
	}
	current.Hub, current.NextHub = current.NextHub, tunnel.Address{}
	if l.save(current) != nil {
		moved.Close()
		return
	}
	l.replaceTunnel(moved, current).Close()
}
