package control_test

import (
	"context"
	"encoding/json"
	"errors"
	"net"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"

	"github.com/TheArctesian/anima/link/control"
)

type sum struct {
	A, B int
}

var verbs = map[string]control.Verb{
	"add": func(_ context.Context, args json.RawMessage) (any, error) {
		var in sum
		if err := json.Unmarshal(args, &in); err != nil {
			return nil, &control.Error{Code: control.CodeBadRequest, Message: "add needs A and B"}
		}
		return in.A + in.B, nil
	},
	"refuse": func(context.Context, json.RawMessage) (any, error) {
		return nil, &control.Error{Code: "not_permitted", Message: "the device is not permitted"}
	},
	"break": func(context.Context, json.RawMessage) (any, error) {
		return nil, errors.New("detail that must stay inside: privkey:abc")
	},
}

// serve starts a control server on a socket and returns a connection to it.
func serve(t *testing.T) (conn net.Conn, stopped chan error, stop context.CancelFunc) {
	t.Helper()
	dir, err := os.MkdirTemp("/tmp", "anima-control-")
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { os.RemoveAll(dir) })
	listener, err := net.Listen("unix", filepath.Join(dir, "c.sock"))
	if err != nil {
		t.Fatal(err)
	}
	ctx, stop := context.WithCancel(context.Background())
	t.Cleanup(stop)
	stopped = make(chan error, 1)
	go func() { stopped <- control.Serve(ctx, listener, verbs) }()
	conn, err = net.Dial("unix", listener.Addr().String())
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { conn.Close() })
	return conn, stopped, stop
}

func TestAVerbGetsItsArgumentsAndItsResultComesBack(t *testing.T) {
	conn, _, _ := serve(t)
	for _, in := range []sum{{2, 3}, {40, 2}} {
		var got int
		if err := control.Call(conn, "add", in, &got); err != nil || got != in.A+in.B {
			t.Errorf("add %+v = %d, %v", in, got, err)
		}
	}
}

func TestARefusalComesBackWithItsCode(t *testing.T) {
	conn, _, _ := serve(t)
	for verb, want := range map[string]string{"refuse": "not_permitted", "missing": control.CodeUnknownVerb} {
		var told *control.Error
		if err := control.Call(conn, verb, nil, nil); !errors.As(err, &told) || told.Code != want {
			t.Errorf("%s: err = %v, want code %s", verb, err, want)
		}
	}
}

func TestLNK008AnErrorThatIsNotARefusalKeepsItsDetailInside(t *testing.T) {
	conn, _, _ := serve(t)
	var told *control.Error
	err := control.Call(conn, "break", nil, nil)
	if !errors.As(err, &told) || told.Code != control.CodeFailed {
		t.Fatalf("err = %v, want code %s", err, control.CodeFailed)
	}
	if strings.Contains(told.Message, "privkey") || strings.Contains(told.Message, "detail") {
		t.Errorf("the reply holds the inner error: %q", told.Message)
	}
}

func TestALineThatIsNotJSONIsRefusedAndTheConnectionStaysUsable(t *testing.T) {
	conn, _, _ := serve(t)
	if _, err := conn.Write([]byte("not json\n")); err != nil {
		t.Fatal(err)
	}
	line := make([]byte, 256)
	n, err := conn.Read(line)
	if err != nil || !strings.Contains(string(line[:n]), control.CodeBadRequest) {
		t.Fatalf("reply to a bad line = %q, %v", line[:n], err)
	}
	var got int
	if err := control.Call(conn, "add", sum{1, 1}, &got); err != nil || got != 2 {
		t.Errorf("after a bad line: add = %d, %v", got, err)
	}
}

func TestServeStopsWithItsContext(t *testing.T) {
	_, stopped, stop := serve(t)
	stop()
	select {
	case err := <-stopped:
		if err != nil {
			t.Errorf("Serve = %v, want nil after a stop", err)
		}
	case <-time.After(2 * time.Second):
		t.Fatal("Serve still runs 2 s after its context ended")
	}
}
