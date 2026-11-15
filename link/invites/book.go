package invites

import (
	"sort"
	"time"

	"github.com/TheArctesian/anima/link/tunnel"
)

// Life is how long an invite can be used (LNK-003, as MET-012).
const Life = 15 * time.Minute

// Refusal is a rule of the invites that a request broke.
type Refusal struct{ code, text string }

func (r *Refusal) Error() string { return r.text }

// Code names the rule for a program.
func (r *Refusal) Code() string { return r.code }

// The rules of the invites.
var (
	ErrBadInvite = &Refusal{"bad_invite", "the text is not an invite"}
	ErrNotOpen   = &Refusal{"invite_not_open", "no open invite has this key: it is used, expired, cancelled or wrong"}
)

// Record is what the hub keeps of one invite. It holds no secret: Key is
// the public half of the invite's temporary key.
type Record struct {
	Key tunnel.DeviceKey `json:"key"`
	// Name is the name that the new device gets.
	Name    string    `json:"name"`
	Expires time.Time `json:"expires"`
}

// book is the set of open invites, by temporary key. An invite leaves the
// book when it is used, cancelled or expired.
type book map[tunnel.DeviceKey]Record

// open reports whether a temporary key belongs to an invite that can be used.
func (b book) open(key tunnel.DeviceKey, now time.Time) bool {
	record, known := b[key]
	return known && now.Before(record.Expires)
}

// without returns the book with one invite less: the invite is used or
// cancelled. It refuses a key with no open invite.
func (b book) without(key tunnel.DeviceKey, now time.Time) (book, Record, error) {
	if !b.open(key, now) {
		return nil, Record{}, ErrNotOpen
	}
	return b.keeping(func(each Record) bool { return each.Key != key }), b[key], nil
}

// with returns the book with one more invite, and with no expired invite.
func (b book) with(record Record, now time.Time) book {
	next := b.keeping(func(each Record) bool { return now.Before(each.Expires) })
	next[record.Key] = record
	return next
}

func (b book) keeping(keep func(Record) bool) book {
	next := book{}
	for key, record := range b {
		if keep(record) {
			next[key] = record
		}
	}
	return next
}

// sorted returns the invites that are open now, the first to expire first.
func (b book) sorted(now time.Time) []Record {
	all := []Record{}
	for _, record := range b {
		if now.Before(record.Expires) {
			all = append(all, record)
		}
	}
	sort.Slice(all, func(i, j int) bool { return all[i].Expires.Before(all[j].Expires) })
	return all
}
