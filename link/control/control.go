// Package control is how the operator, and later the application and the
// hub, tell a running anima-link what to do: one JSON request on a line,
// one JSON reply on a line, over a socket that only the owner can reach.
package control

import (
	"bufio"
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"net"
)

// Error is a refusal that crosses the socket. Code is for programs and
// Message is for the operator. Neither may hold a secret (LNK-008).
type Error struct {
	Code    string `json:"code"`
	Message string `json:"message"`
}

func (e *Error) Error() string { return e.Message }

// The codes that this package makes itself. Verbs add their own.
const (
	CodeUnknownVerb = "unknown_verb"
	CodeBadRequest  = "bad_request"
	CodeFailed      = "failed"
)

// Verb does one kind of request. args is the request's JSON arguments. An
// error that is not an *Error reaches the caller as CodeFailed with a fixed
// message, so that no detail leaves by accident.
type Verb func(ctx context.Context, args json.RawMessage) (any, error)

// Arguments reads the arguments of a request into value. A field that value
// does not have is a mistake of the caller, and is refused.
func Arguments(args json.RawMessage, value any) error {
	decoder := json.NewDecoder(bytes.NewReader(args))
	decoder.DisallowUnknownFields()
	if decoder.Decode(value) != nil {
		return &Error{CodeBadRequest, "the arguments of this verb are not valid"}
	}
	return nil
}

type request struct {
	Verb string          `json:"verb"`
	Args json.RawMessage `json:"args,omitempty"`
}

type reply struct {
	OK     bool            `json:"ok"`
	Result json.RawMessage `json:"result,omitempty"`
	Error  *Error          `json:"error,omitempty"`
}

// maxLine is the largest request or reply. Requests are small; a list of
// devices is the largest reply.
const maxLine = 1 << 20

// Serve answers requests on each connection of the listener until ctx ends.
func Serve(ctx context.Context, listener net.Listener, verbs map[string]Verb) error {
	go func() {
		<-ctx.Done()
		listener.Close()
	}()
	for {
		conn, err := listener.Accept()
		if err != nil {
			if ctx.Err() != nil {
				return nil
			}
			return err
		}
		go answer(ctx, conn, verbs)
	}
}

func answer(ctx context.Context, conn net.Conn, verbs map[string]Verb) {
	defer conn.Close()
	lines := bufio.NewScanner(conn)
	lines.Buffer(make([]byte, 0, 4096), maxLine)
	encoder := json.NewEncoder(conn)
	for lines.Scan() {
		if encoder.Encode(run(ctx, lines.Bytes(), verbs)) != nil {
			return
		}
	}
}

func run(ctx context.Context, line []byte, verbs map[string]Verb) reply {
	var req request
	if json.Unmarshal(line, &req) != nil {
		return refusal(&Error{CodeBadRequest, "the request is not valid JSON"})
	}
	verb, known := verbs[req.Verb]
	if !known {
		return refusal(&Error{CodeUnknownVerb, "no such verb"})
	}
	result, err := verb(ctx, req.Args)
	if err != nil {
		return refusal(err)
	}
	text, err := json.Marshal(result)
	if err != nil {
		return refusal(err)
	}
	return reply{OK: true, Result: text}
}

func refusal(err error) reply {
	var told *Error
	if !errors.As(err, &told) {
		told = &Error{CodeFailed, "the request failed; see the log of anima-link"}
	}
	return reply{Error: told}
}

// Call sends one request on conn and reads its reply into result (nil to
// drop it). A refusal comes back as an *Error.
func Call(conn net.Conn, verb string, args, result any) error {
	argsText, err := json.Marshal(args)
	if err != nil {
		return err
	}
	if err := json.NewEncoder(conn).Encode(request{Verb: verb, Args: argsText}); err != nil {
		return err
	}
	lines := bufio.NewScanner(conn)
	lines.Buffer(make([]byte, 0, 4096), maxLine)
	if !lines.Scan() {
		if err := lines.Err(); err != nil {
			return err
		}
		return &Error{CodeFailed, "anima-link closed the connection with no reply"}
	}
	var got reply
	if err := json.Unmarshal(lines.Bytes(), &got); err != nil {
		return &Error{CodeFailed, "the reply is not valid JSON"}
	}
	if !got.OK {
		if got.Error == nil {
			return &Error{CodeFailed, "the reply has no error"}
		}
		return got.Error
	}
	if result == nil {
		return nil
	}
	return json.Unmarshal(got.Result, result)
}
