package unit

import (
	"encoding/xml"
	"strings"
	"testing"
)

func TestWindowsTaskIsPerUserWithoutAPasswordOrElevatedRights(t *testing.T) {
	on := System{GOOS: "windows", SID: "S-1-5-21-123-456-789-1000", Windows: `C:\Windows`, ConfigHome: `C:\Users\A & B\AppData\Local`}
	settings := Settings{Program: `C:\Users\A & B\AppData\Local\Anima\bin\anima-link.exe`, State: `C:\Users\A & B\AppData\Local\Anima\link`, Port: 41001}
	plan, err := PlanFor(settings, on)
	if err != nil {
		t.Fatal(err)
	}
	var parsed struct {
		Command   string `xml:"Actions>Exec>Command"`
		Arguments string `xml:"Actions>Exec>Arguments"`
		User      string `xml:"Principals>Principal>UserId"`
		Logon     string `xml:"Principals>Principal>LogonType"`
		Level     string `xml:"Principals>Principal>RunLevel"`
		Limit     string `xml:"Settings>ExecutionTimeLimit"`
	}
	if err := xml.Unmarshal([]byte(plan.Content), &parsed); err != nil {
		t.Fatal(err)
	}
	if parsed.Command != settings.Program || parsed.User != on.SID || parsed.Logon != "InteractiveToken" || parsed.Level != "LeastPrivilege" || parsed.Limit != "PT0S" {
		t.Fatalf("wrong task identity or lifetime: %+v", parsed)
	}
	if !strings.Contains(parsed.Arguments, `"`+settings.State+`"`) {
		t.Fatal("a path containing spaces must be one argument")
	}
	if len(plan.Start) != 3 || plan.Start[1].MayFail || plan.Start[2].MayFail {
		t.Fatal("registration and start must report errors")
	}
	for _, step := range append(plan.Start, plan.Stop...) {
		for _, arg := range step.Args {
			if arg == "/RP" || arg == "/RU" {
				t.Fatal("task must not ask for another account or password")
			}
		}
	}
}

func TestWindowsTaskRejectsRelativeNetworkAndAmbiguousPaths(t *testing.T) {
	for _, value := range []string{`relative`, `C:relative`, `\\server\share\link.exe`, `C:\a\..\b`, `C:\a:stream`, `C:\a"b`, `C:\%TEMP%\b`} {
		if windowsPath(value) {
			t.Errorf("accepted %q", value)
		}
	}
	if !windowsPath(`C:\Users\A & B\link.exe`) {
		t.Fatal("ordinary paths with spaces and XML metacharacters must work")
	}
}
