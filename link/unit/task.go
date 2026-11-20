package unit

import (
	"encoding/xml"
	"path"
	"regexp"
	"strings"
)

var sidPattern = regexp.MustCompile(`^S-1-[0-9]+(-[0-9]+)+$`)

// windowsPath accepts local absolute paths. XML escaping protects the task document;
// quotes, variables, alternate streams and relative components are refused.
func windowsPath(value string) bool {
	normalized := strings.ReplaceAll(value, `\`, "/")
	if len(normalized) < 3 || normalized[1:3] != ":/" || path.Clean(normalized) != normalized {
		return false
	}
	drive := normalized[0]
	if !((drive >= 'A' && drive <= 'Z') || (drive >= 'a' && drive <= 'z')) {
		return false
	}
	for _, c := range normalized[2:] {
		if c < 32 || c == 127 || strings.ContainsRune(`"%:`, c) {
			return false
		}
	}
	return true
}

func escaped(value string) string {
	var out strings.Builder
	_ = xml.EscapeText(&out, []byte(value))
	return out.String()
}

func taskArguments(args []string) string {
	quoted := make([]string, len(args))
	for i, arg := range args {
		// Paths cannot contain quotes. Double trailing backslashes before the closing quote.
		suffix := len(arg) - len(strings.TrimRight(arg, `\`))
		quoted[i] = `"` + arg + strings.Repeat(`\`, suffix) + `"`
	}
	return strings.Join(quoted, " ")
}

func windowsPlace(on System) (Plan, error) {
	if !sidPattern.MatchString(on.SID) || !windowsPath(on.ConfigHome) || !windowsPath(on.Windows) {
		return Plan{}, refused("Windows needs this user's SID, local application folder and system directory")
	}
	name := `Anima.ing-Link-` + on.SID
	file := strings.TrimRight(on.ConfigHome, `\/`) + `\Anima\units\link.xml`
	program := strings.TrimRight(on.Windows, `\/`) + `\System32\schtasks.exe`
	step := func(mayFail bool, args ...string) Step { return Step{Program: program, Args: args, MayFail: mayFail} }
	return Plan{File: file,
		Start: []Step{step(true, "/End", "/TN", name), step(false, "/Create", "/TN", name, "/XML", file, "/F"), step(false, "/Run", "/TN", name)},
		Stop:  []Step{step(true, "/End", "/TN", name), step(true, "/Delete", "/TN", name, "/F")},
	}, nil
}

func windowsTask(s Settings, on System) (Plan, error) {
	if !windowsPath(s.Program) || !windowsPath(s.State) || s.Port == 0 || (s.DaemonSocket != "" && !windowsPath(s.DaemonSocket)) {
		return Plan{}, refused("Windows needs local absolute program/state paths and a fixed port")
	}
	plan, err := windowsPlace(on)
	if err != nil {
		return Plan{}, err
	}
	sid := escaped(on.SID)
	plan.Content = `<?xml version="1.0"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
<RegistrationInfo><Description>Anima.ing device connection for the signed-in user.</Description></RegistrationInfo>
<Triggers><LogonTrigger><Enabled>true</Enabled><UserId>` + sid + `</UserId></LogonTrigger></Triggers>
<Principals><Principal id="User"><UserId>` + sid + `</UserId><LogonType>InteractiveToken</LogonType><RunLevel>LeastPrivilege</RunLevel></Principal></Principals>
<Settings><MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy><DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries><StopIfGoingOnBatteries>false</StopIfGoingOnBatteries><AllowHardTerminate>true</AllowHardTerminate><StartWhenAvailable>true</StartWhenAvailable><RunOnlyIfNetworkAvailable>false</RunOnlyIfNetworkAvailable><AllowStartOnDemand>true</AllowStartOnDemand><Enabled>true</Enabled><ExecutionTimeLimit>PT0S</ExecutionTimeLimit><RestartOnFailure><Interval>PT1M</Interval><Count>3</Count></RestartOnFailure></Settings>
<Actions Context="User"><Exec><Command>` + escaped(s.Program) + `</Command><Arguments>` + escaped(taskArguments(s.arguments()[1:])) + `</Arguments></Exec></Actions>
</Task>
`
	return plan, nil
}
