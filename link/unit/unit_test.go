package unit

import (
	"errors"
	"strings"
	"testing"
)

var settings = Settings{
	Program: "/Users/me/.local/bin/anima-link", State: "/Users/me/Library/Application Support/ing.anima/link",
	Port: 41001, DaemonSocket: "/Users/me/Library/Application Support/anima-daemon/link.sock",
}

var mac = System{GOOS: "darwin", UID: 501, Home: "/Users/me"}

func commandLines(steps []Step) []string {
	lines := make([]string, 0, len(steps))
	for _, step := range steps {
		lines = append(lines, step.Program+" "+strings.Join(step.Args, " "))
	}
	return lines
}

func equal(t *testing.T, what string, got, want []string) {
	t.Helper()
	if strings.Join(got, "\n") != strings.Join(want, "\n") {
		t.Errorf("%s:\n%s\nwant:\n%s", what, strings.Join(got, "\n"), strings.Join(want, "\n"))
	}
}

func TestHMD058TheLaunchAgentRunsTheDeviceLinkAtTheLoginAndKeepsItRunning(t *testing.T) {
	plan, err := PlanFor(settings, mac)
	if err != nil {
		t.Fatal(err)
	}
	if plan.File != "/Users/me/Library/LaunchAgents/ing.anima.link.plist" {
		t.Errorf("file = %q", plan.File)
	}
	for _, want := range []string{
		"<string>ing.anima.link</string>",
		"<string>/Users/me/.local/bin/anima-link</string>\n        <string>device</string>\n        <string>-state</string>\n" +
			"        <string>/Users/me/Library/Application Support/ing.anima/link</string>\n        <string>-listen</string>\n" +
			"        <string>127.0.0.1:41001</string>\n        <string>-daemon-socket</string>\n" +
			"        <string>/Users/me/Library/Application Support/anima-daemon/link.sock</string>\n    </array>",
		"<key>RunAtLoad</key>\n    <true/>", "<key>KeepAlive</key>\n    <true/>",
	} {
		if !strings.Contains(plan.Content, want) {
			t.Errorf("the LaunchAgent does not hold %q:\n%s", want, plan.Content)
		}
	}
	if strings.Contains(plan.Content, "stop-with-stdin") {
		t.Error("a unit has no standard input to stop with")
	}
	equal(t, "start", commandLines(plan.Start), []string{
		"/bin/launchctl bootout gui/501/ing.anima.link",
		"/bin/launchctl bootstrap gui/501 /Users/me/Library/LaunchAgents/ing.anima.link.plist",
	})
	if !plan.Start[0].MayFail || plan.Start[1].RetryAfter == 0 {
		t.Error("the bootout of a unit that is not loaded may fail, and the bootstrap is tried once more")
	}
	equal(t, "stop", commandLines(plan.Stop), []string{"/bin/launchctl bootout gui/501/ing.anima.link"})
}

func TestHMD058TheSystemdUserUnitRunsTheDeviceLinkWithNoSocketWhenNoneIsGiven(t *testing.T) {
	linux := Settings{Program: "/home/me/.local/bin/anima-link", State: "/home/me/.local/share/ing.anima/link", Port: 41001}
	plan, err := PlanFor(linux, System{GOOS: "linux", UID: 1000, Home: "/home/me"})
	if err != nil {
		t.Fatal(err)
	}
	if plan.File != "/home/me/.config/systemd/user/anima-link.service" {
		t.Errorf("file = %q", plan.File)
	}
	exec := `ExecStart="/home/me/.local/bin/anima-link" "device" "-state" "/home/me/.local/share/ing.anima/link" "-listen" "127.0.0.1:41001"` + "\n"
	for _, want := range []string{exec, "Restart=always\n", "WantedBy=default.target\n"} {
		if !strings.Contains(plan.Content, want) {
			t.Errorf("the unit does not hold %q:\n%s", want, plan.Content)
		}
	}
	equal(t, "start", commandLines(plan.Start), []string{
		"/usr/bin/systemctl --user daemon-reload",
		"/usr/bin/systemctl --user enable anima-link.service",
		"/usr/bin/systemctl --user restart anima-link.service",
	})
	equal(t, "stop", commandLines(plan.Stop), []string{"/usr/bin/systemctl --user disable --now anima-link.service"})
	if got := plan.Start[0].Env; len(got) != 1 || got[0] != "XDG_RUNTIME_DIR=/run/user/1000" {
		t.Errorf("systemctl gets %v, want the runtime folder of the user", got)
	}
}

func TestTheSystemdUnitGoesToTheConfigFolderThatTheUserSet(t *testing.T) {
	linux := Settings{Program: "/home/me/.local/bin/anima-link", State: "/s", Port: 1}
	plan, err := PlanFor(linux, System{GOOS: "linux", UID: 1000, Home: "/home/me", ConfigHome: "/home/me/conf", RuntimeDir: "/run/user/7"})
	if err != nil {
		t.Fatal(err)
	}
	if plan.File != "/home/me/conf/systemd/user/anima-link.service" || plan.Stop[0].Env[0] != "XDG_RUNTIME_DIR=/run/user/7" {
		t.Errorf("file = %q, env = %v", plan.File, plan.Stop[0].Env)
	}
}

func TestHMD058AValueThatCouldBreakOutOfTheUnitFileIsRefused(t *testing.T) {
	change := func(edit func(*Settings)) Settings {
		changed := settings
		edit(&changed)
		return changed
	}
	cases := map[string]Settings{
		"a relative program":          change(func(s *Settings) { s.Program = "anima-link" }),
		"a program in an application": change(func(s *Settings) { s.Program = "/Applications/Anima.ing.app/Contents/MacOS/anima-link" }),
		"a program in an AppImage":    change(func(s *Settings) { s.Program = "/tmp/.mount_AnimaXy/usr/bin/anima-link" }),
		"a program on a disk image":   change(func(s *Settings) { s.Program = "/Volumes/Anima.ing/anima-link" }),
		"a tag in the state path":     change(func(s *Settings) { s.State = "/s</string><string>-x" }),
		"a quote in the state path":   change(func(s *Settings) { s.State = `/s" "-x` }),
		"a line end in the socket":    change(func(s *Settings) { s.DaemonSocket = "/s\nExecStartPre=/bin/x" }),
		"a percent sign":              change(func(s *Settings) { s.State = "/s/%h" }),
		"a path with dots":            change(func(s *Settings) { s.State = "/s/../etc" }),
		"no fixed port":               change(func(s *Settings) { s.Port = 0 }),
	}
	for name, given := range cases {
		for _, on := range []System{mac, {GOOS: "linux", UID: 1000, Home: "/home/me"}} {
			if _, err := PlanFor(given, on); !errors.Is(err, ErrRefused) {
				t.Errorf("%s on %s: err = %v, want ErrRefused", name, on.GOOS, err)
			}
		}
	}
}

func TestASystemWithNoUnitOfTheUserIsRefused(t *testing.T) {
	if _, err := PlanFor(settings, System{GOOS: "windows", UID: 1, Home: "/Users/me"}); !errors.Is(err, ErrRefused) {
		t.Errorf("err = %v, want ErrRefused", err)
	}
	if _, err := Place(System{GOOS: "linux", UID: 1, Home: "/home/me", ConfigHome: "relative"}); !errors.Is(err, ErrRefused) {
		t.Errorf("a relative XDG_CONFIG_HOME: err = %v, want ErrRefused", err)
	}
}
