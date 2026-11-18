// Package dial is how the hub reaches a local port of a computer on its
// link (HMD-048). The hub cannot open a stream to a device, so the device
// opens it:
//
//  1. The hub link writes "open <port> <id>" on the device's stream of the
//     reverse port (package presence). The id is 16 random bytes in hex.
//  2. The device connects to the port on its own loopback address, only
//     when the port is in its list of open ports (package ports, HMD-049).
//  3. The device opens a new stream to the open port of the hub
//     (tunnel.OpenPort) and writes "<id> ok", then the bytes of the local
//     connection; or "<id> refused" (the port is not in the list) or
//     "<id> closed" (nothing listens on it), and closes the stream.
//
// The hub gets the connection through its dial socket: the request line
// "dial <link address> <port>", then the line "ok" and the bytes in both
// directions, or one word line and the end of the connection.
//
// Each line ends with a newline. No line holds a secret, but an id is
// known only to the hub and the one device, and is never logged.
package dial

import (
	"crypto/rand"
	"encoding/hex"
	"errors"
	"io"
	"net/netip"
	"strconv"
	"strings"
)

// The words of the answers. The device says the first three; the dial
// socket also says the others.
const (
	WordOK           = "ok"
	WordRefused      = "refused"
	WordClosed       = "closed"
	WordNotConnected = "not_connected"
	WordTimeout      = "timeout"
	WordBadRequest   = "bad_request"
)

// Errors of this package: why the hub got no connection. Word gives the
// word of each for the dial socket.
var (
	ErrNotConnected = errors.New("dial: the device has no stream on the reverse port")
	ErrRefused      = errors.New("dial: the port is not open on the device")
	ErrClosed       = errors.New("dial: nothing listens on the port of the device")
	ErrTimeout      = errors.New("dial: the device did not answer in time")
	ErrBadRequest   = errors.New("dial: the request is not valid")
)

// maxLine is the longest line of this package, without its newline.
const maxLine = 128

// idBytes is the number of random bytes of an id.
const idBytes = 16

// newID makes the id of one request.
func newID() string {
	id := make([]byte, idBytes)
	_, _ = rand.Read(id)
	return hex.EncodeToString(id)
}

func validID(id string) bool {
	if len(id) != 2*idBytes {
		return false
	}
	_, err := hex.DecodeString(id)
	return err == nil && strings.ToLower(id) == id
}

// parsePort reads a port number in decimal with no sign and no leading
// zero. Port 0 is not a port.
func parsePort(text string) (uint16, bool) {
	if text == "" || text[0] == '0' || text[0] == '+' {
		return 0, false
	}
	port, err := strconv.ParseUint(text, 10, 16)
	return uint16(port), err == nil
}

// openLine is the request of the hub on the reverse port.
func openLine(port uint16, id string) string {
	return "open " + strconv.Itoa(int(port)) + " " + id + "\n"
}

// Request is one request of the hub for a local port of the device.
type Request struct {
	Port uint16
	id   string
}

// ParseRequest reads a line of the hub on the reverse port, without its
// newline. It reports false for a line that is not an open request.
func ParseRequest(line string) (Request, bool) {
	fields := strings.Split(line, " ")
	if len(fields) != 3 || fields[0] != "open" || !validID(fields[2]) {
		return Request{}, false
	}
	port, valid := parsePort(fields[1])
	return Request{Port: port, id: fields[2]}, valid
}

// answerLine is the first line of the device on the open port.
func answerLine(id, word string) string { return id + " " + word + "\n" }

// parseAnswer reads the first line of the device on the open port.
func parseAnswer(line string) (id, word string, valid bool) {
	id, word, found := strings.Cut(line, " ")
	switch {
	case !found || !validID(id):
		return "", "", false
	case word == WordOK, word == WordRefused, word == WordClosed:
		return id, word, true
	default:
		return "", "", false
	}
}

// parseDial reads the request line of the dial socket, without its
// newline.
func parseDial(line string) (netip.Addr, uint16, bool) {
	fields := strings.Split(line, " ")
	if len(fields) != 3 || fields[0] != "dial" {
		return netip.Addr{}, 0, false
	}
	device, err := netip.ParseAddr(fields[1])
	port, valid := parsePort(fields[2])
	return device, port, err == nil && valid
}

// Word is the word that the dial socket says for an error of Open. An
// error of no other kind is a timeout: the hub link has stopped or the
// stream ended.
func Word(err error) string {
	switch {
	case errors.Is(err, ErrNotConnected):
		return WordNotConnected
	case errors.Is(err, ErrRefused):
		return WordRefused
	case errors.Is(err, ErrClosed):
		return WordClosed
	case errors.Is(err, ErrBadRequest):
		return WordBadRequest
	default:
		return WordTimeout
	}
}

// readLine reads one line of at most maxLine bytes, one byte at a time: the
// bytes after the line belong to the connection that follows, and a
// buffered reader would take them.
func readLine(stream io.Reader) (string, bool) {
	line := make([]byte, 0, maxLine)
	one := make([]byte, 1)
	for len(line) <= maxLine {
		if _, err := io.ReadFull(stream, one); err != nil {
			return "", false
		}
		if one[0] == '\n' {
			return string(line), true
		}
		line = append(line, one[0])
	}
	return "", false
}
