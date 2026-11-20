package unit

import (
	"context"
	"encoding/json"
	"flag"
	"fmt"
	"io"
	"net/netip"
	"os"
	"path/filepath"
	"runtime"
)

// shown is the answer of "unit show".
type shown struct {
	Installed bool   `json:"installed"`
	File      string `json:"file"`
}

// here is this computer and this user.
func here() (System, error) {
	home, err := os.UserHomeDir()
	if err != nil {
		return System{}, refused("no home folder: set HOME")
	}
	return System{
		GOOS: runtime.GOOS, UID: os.Getuid(), Home: home,
		ConfigHome: os.Getenv("XDG_CONFIG_HOME"), RuntimeDir: os.Getenv("XDG_RUNTIME_DIR"),
	}, nil
}

// settingsFrom reads the flags of "unit install". The program of the unit
// is the program that runs now, by its real path.
func settingsFrom(args []string) (Settings, error) {
	flags := flag.NewFlagSet("unit install", flag.ContinueOnError)
	flags.SetOutput(io.Discard)
	state := flags.String("state", "", "the state directory")
	listen := flags.String("listen", "", "the local loopback address, 127.0.0.1:PORT")
	socket := flags.String("daemon-socket", "", "the path of a Unix socket for the metrics daemon")
	if err := flags.Parse(args); err != nil || flags.NArg() != 0 || *state == "" {
		return Settings{}, refused("use: unit install -state DIR -listen 127.0.0.1:PORT [-daemon-socket SOCKET-PATH]")
	}
	address, err := netip.ParseAddrPort(*listen)
	if err != nil || address.Addr() != netip.AddrFrom4([4]byte{127, 0, 0, 1}) {
		return Settings{}, refused("-listen must be 127.0.0.1:PORT")
	}
	program, err := os.Executable()
	if err == nil {
		program, err = filepath.EvalSymlinks(program)
	}
	if err != nil {
		return Settings{}, refused("the path of this program cannot be read")
	}
	return Settings{Program: program, State: *state, Port: address.Port(), DaemonSocket: *socket}, nil
}

func install(ctx context.Context, args []string, run Runner) error {
	settings, err := settingsFrom(args)
	if err != nil {
		return err
	}
	on, err := here()
	if err != nil {
		return err
	}
	plan, err := PlanFor(settings, on)
	if err != nil {
		return err
	}
	if err := CheckProgram(settings.Program, on.UID); err != nil {
		return err
	}
	return Install(ctx, plan, run)
}

// Command does "anima-link unit install|remove|show" for this user. It
// writes the answer of "show" to out as one line of JSON.
func Command(ctx context.Context, args []string, out io.Writer) error {
	if len(args) == 0 {
		return refused("use: unit install|remove|show")
	}
	if args[0] == "install" {
		return install(ctx, args[1:], Run)
	}
	on, err := here()
	if err != nil {
		return err
	}
	plan, err := Place(on)
	if err != nil || len(args) != 1 {
		return refused("use: unit remove, or unit show")
	}
	switch args[0] {
	case "remove":
		return Remove(ctx, plan, Run)
	case "show":
		answer, err := json.Marshal(shown{Installed: Installed(plan), File: plan.File})
		if err != nil {
			return err
		}
		_, err = fmt.Fprintln(out, string(answer))
		return err
	default:
		return refused("use: unit install|remove|show")
	}
}
