// Package unit is the unit of the user that keeps the device link running
// with no application open (HMD-058, R-240): a LaunchAgent on macOS, a
// systemd user unit on Linux. It writes the file, starts it and removes it.
// Nothing here runs as root, and nothing asks for a password.
//
// This file is pure: what the unit says, where its file is, and which
// commands start and stop it. The work on the system is in install.go.
package unit

import (
	"errors"
	"fmt"
	"path/filepath"
	"strconv"
	"strings"
	"time"
)

// ErrRefused means that the unit cannot be made from what was given. The
// message says which value is wrong.
var ErrRefused = errors.New("unit: refused")

// Label is the name of the LaunchAgent, and ServiceName the name of the
// systemd unit.
const (
	Label       = "ing.anima.link"
	ServiceName = "anima-link.service"
)

// Settings is what the unit runs: `Program device -state State -listen
// 127.0.0.1:Port [-daemon-socket DaemonSocket]`.
type Settings struct {
	// Program is the absolute path of anima-link. It must be a file that
	// stays: not a file inside an application or a mounted image.
	Program string
	// State is the absolute path of the state directory of the device.
	State string
	// Port is the loopback port that the link listens on. A unit has a fixed
	// port, so that the application finds the link again.
	Port uint16
	// DaemonSocket is the absolute path of the socket of the metrics daemon;
	// empty means no socket.
	DaemonSocket string
}

// System is the computer and the user that the unit is for.
type System struct {
	// GOOS is a value of runtime.GOOS.
	GOOS string
	UID  int
	// Home is the home folder of the user, and ConfigHome the value of
	// XDG_CONFIG_HOME (empty when it is not set).
	Home       string
	ConfigHome string
	// RuntimeDir is the value of XDG_RUNTIME_DIR (empty when it is not set).
	RuntimeDir string
}

// Step is one fixed command: an absolute program, no shell and no input.
type Step struct {
	Program string
	Args    []string
	Env     []string
	// MayFail means that a failure is expected and fine: to stop a unit
	// that is not loaded.
	MayFail bool
	// RetryAfter, when not zero, runs the command once more after this
	// pause when its first run fails.
	RetryAfter time.Duration
}

// Plan is the file of the unit and the commands around it.
type Plan struct {
	File    string
	Content string
	// Start runs after the file is written, Stop before it is deleted, and
	// After after it is deleted.
	Start []Step
	Stop  []Step
	After []Step
}

func refused(format string, args ...any) error {
	return fmt.Errorf("%w: %s", ErrRefused, fmt.Sprintf(format, args...))
}

// plainPath reports whether a path is absolute, clean, and holds no
// character that could end a value in a unit file: a control character, a
// quote, a backslash, a percent sign, a dollar sign or an angle bracket.
func plainPath(path string) bool {
	if !filepath.IsAbs(path) || filepath.Clean(path) != path {
		return false
	}
	for _, char := range path {
		if char < 0x20 || char == 0x7f || strings.ContainsRune(`"'\%$<>&`, char) {
			return false
		}
	}
	return true
}

// carried reports whether a path is inside an application bundle or a
// mounted image: such a file is gone or changed after an update or a move.
func carried(path string) bool {
	return strings.Contains(path, ".app/") || strings.HasPrefix(path, "/Volumes/") ||
		strings.HasPrefix(path, "/tmp/.mount_") || strings.Contains(path, "/AppTranslocation/")
}

func (s Settings) check() error {
	switch {
	case !plainPath(s.Program):
		return refused("the program path must be absolute and plain")
	case carried(s.Program):
		return refused("the program is inside an application or a mounted image: copy it to a folder that stays, like ~/.local/bin")
	case !plainPath(s.State):
		return refused("the state directory must be an absolute, plain path")
	case s.Port == 0:
		return refused("the unit needs a fixed port: give -listen 127.0.0.1:PORT")
	case s.DaemonSocket != "" && !plainPath(s.DaemonSocket):
		return refused("the path of the daemon socket must be absolute and plain")
	}
	return nil
}

// arguments is the command line of the link, with the program first.
func (s Settings) arguments() []string {
	args := []string{s.Program, "device", "-state", s.State, "-listen", "127.0.0.1:" + strconv.Itoa(int(s.Port))}
	if s.DaemonSocket != "" {
		args = append(args, "-daemon-socket", s.DaemonSocket)
	}
	return args
}

// launchAgent is the text of the LaunchAgent. plainPath lets no character
// through that XML must escape.
func launchAgent(s Settings) string {
	var text strings.Builder
	text.WriteString(`<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<!-- The device link of Anima.ing as a LaunchAgent (HMD-058). "anima-link unit install" writes this file. -->
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>` + Label + `</string>
    <key>ProgramArguments</key>
    <array>
`)
	for _, arg := range s.arguments() {
		text.WriteString("        <string>" + arg + "</string>\n")
	}
	text.WriteString(`    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>ThrottleInterval</key>
    <integer>10</integer>
    <key>ProcessType</key>
    <string>Background</string>
</dict>
</plist>
`)
	return text.String()
}

// systemdUnit is the text of the systemd user unit. Each argument is in
// double quotes; plainPath lets no character through that ends them.
func systemdUnit(s Settings) string {
	quoted := make([]string, 0, 8)
	for _, arg := range s.arguments() {
		quoted = append(quoted, `"`+arg+`"`)
	}
	return `# The device link of Anima.ing as a systemd user unit (HMD-058). "anima-link unit install" writes this file.
[Unit]
Description=Anima.ing link: connects this computer to its hub
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=` + strings.Join(quoted, " ") + `
Restart=always
RestartSec=10
NoNewPrivileges=yes
LimitCORE=0

[Install]
WantedBy=default.target
`
}

const (
	launchctl      = "/bin/launchctl"
	systemctl      = "/usr/bin/systemctl"
	bootstrapRetry = time.Second
)

func darwinPlace(on System) Plan {
	file := filepath.Join(on.Home, "Library", "LaunchAgents", Label+".plist")
	domain := "gui/" + strconv.Itoa(on.UID)
	bootout := Step{Program: launchctl, Args: []string{"bootout", domain + "/" + Label}, MayFail: true}
	return Plan{
		File: file,
		// A loaded unit is taken out first, so that a new file or a new program is used.
		Start: []Step{bootout, {Program: launchctl, Args: []string{"bootstrap", domain, file}, RetryAfter: bootstrapRetry}},
		Stop:  []Step{bootout},
	}
}

func linuxPlace(on System) (Plan, error) {
	config := on.ConfigHome
	if config == "" {
		config = filepath.Join(on.Home, ".config")
	}
	runtimeDir := on.RuntimeDir
	if runtimeDir == "" {
		runtimeDir = "/run/user/" + strconv.Itoa(on.UID)
	}
	if !plainPath(config) || !plainPath(runtimeDir) {
		return Plan{}, refused("XDG_CONFIG_HOME and XDG_RUNTIME_DIR must be absolute, plain paths")
	}
	user := func(mayFail bool, args ...string) Step {
		return Step{
			Program: systemctl, Args: append([]string{"--user"}, args...),
			Env: []string{"XDG_RUNTIME_DIR=" + runtimeDir}, MayFail: mayFail,
		}
	}
	return Plan{
		File: filepath.Join(config, "systemd", "user", ServiceName),
		// restart starts a stopped unit and restarts a running one, so a new program is used.
		Start: []Step{user(false, "daemon-reload"), user(false, "enable", ServiceName), user(false, "restart", ServiceName)},
		Stop:  []Step{user(true, "disable", "--now", ServiceName)},
		After: []Step{user(true, "daemon-reload")},
	}, nil
}

// Place is where the unit of the user is on the system on, with the
// commands that start and stop it and no content: enough to remove a unit
// and to see whether one is there. It returns ErrRefused for a system that
// has no unit of the user.
func Place(on System) (Plan, error) {
	if !plainPath(on.Home) {
		return Plan{}, refused("the home folder must be an absolute, plain path")
	}
	switch on.GOOS {
	case "darwin":
		return darwinPlace(on), nil
	case "linux":
		return linuxPlace(on)
	default:
		return Plan{}, refused("no unit of the user on %s", on.GOOS)
	}
}

// PlanFor is the file, the content and the commands of the unit of s on
// the system on. It returns ErrRefused for a value that cannot go into a
// unit, and for a system that has no unit of the user.
func PlanFor(s Settings, on System) (Plan, error) {
	if err := s.check(); err != nil {
		return Plan{}, err
	}
	plan, err := Place(on)
	if err != nil {
		return Plan{}, err
	}
	if on.GOOS == "darwin" {
		plan.Content = launchAgent(s)
	} else {
		plan.Content = systemdUnit(s)
	}
	return plan, nil
}
