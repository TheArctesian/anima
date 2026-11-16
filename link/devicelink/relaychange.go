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
	ctx, cancel := context.WithTimeout(ctx, l.pingLimit)
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
	if err != nil || next == current.NextHub || next == current.Hub || !next.SameHub(current.Hub) {
		return
	}
	current.NextHub = next
	if l.save(current) == nil {
		l.setIdentity(current)
	}
}

// followHub looks for the hub after it has stopped answering at its
// address: first at the address that the hub announced, then at the
// addresses that the hub's domain name holds. When the hub answers at one,
// that address becomes the hub's address.
func (l *Link) followHub(ctx context.Context) {
	current := l.identity()
	if l.save == nil {
		return
	}
	candidates := []tunnel.Address{}
	if !current.NextHub.IsZero() {
		candidates = append(candidates, current.NextHub)
	}
	for _, named := range l.addressesOfName(ctx, current.HubName) {
		if named != current.Hub && named != current.NextHub {
			candidates = append(candidates, named)
		}
	}
	for _, candidate := range candidates {
		if l.moveTo(ctx, candidate) {
			return
		}
	}
}

// moveTo makes an address the hub's address if the hub answers there. The
// address must name the hub that the device joined: the same key and the
// same pre-shared key, on another relay. An address with another key is
// refused, wherever it comes from.
func (l *Link) moveTo(ctx context.Context, address tunnel.Address) bool {
	current := l.identity()
	if !address.SameHub(current.Hub) {
		return false
	}
	moved, err := tunnel.NewDevice(tunnel.DeviceConfig{Key: current.Key, Hub: address, DebugLogf: l.debugLogf})
	if err != nil {
		return false
	}
	ctx, cancel := context.WithTimeout(ctx, l.registerLimit)
	defer cancel()
	if !answersAsHub(ctx, moved) {
		moved.Close()
		return false
	}
	current.Hub, current.NextHub = address, tunnel.Address{}
	if l.save(current) != nil {
		moved.Close()
		return false
	}
	l.replaceTunnel(moved, current).Close()
	return true
}

// answersAsHub reports whether the hub itself answers through a tunnel. An
// answer to Register is not enough: the relay can give it, and so can
// anybody on the relay. A stream needs the tunnel's handshake, which only
// the holder of the hub's private key and pre-shared key can complete.
func answersAsHub(ctx context.Context, device *tunnel.Device) bool {
	if device.Register(ctx) != nil {
		return false
	}
	stream, err := device.Dial(ctx, tunnel.NoticePort)
	if err != nil {
		return false
	}
	defer stream.Close()
	if deadline, ok := ctx.Deadline(); ok {
		_ = stream.SetDeadline(deadline)
	}
	// The hub writes one line and closes. A relay cannot make these bytes.
	_, err = bufio.NewReader(stream).ReadBytes('\n')
	return err == nil
}

// hubNameLabel comes before the hub's domain name in the name of the TXT
// record that holds the hub address.
const hubNameLabel = "_anima-link."

// addressesOfName reads the hub addresses that a domain name holds
// (LNK-007). A name only finds the hub: the hub still admits only the keys
// in its list. The address in the record is public, so its pre-shared key
// protects nothing for a hub with a name.
func (l *Link) addressesOfName(ctx context.Context, name string) []tunnel.Address {
	if name == "" {
		return nil
	}
	ctx, cancel := context.WithTimeout(ctx, l.pingLimit)
	defer cancel()
	records, err := l.lookupTXT(ctx, hubNameLabel+name)
	if err != nil {
		return nil
	}
	found := []tunnel.Address{}
	for _, record := range records {
		if address, err := tunnel.ParseAddress(record); err == nil {
			found = append(found, address)
		}
	}
	return found
}
