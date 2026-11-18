//go:build !darwin || ios

package statedir

// dropInheritedAccess does nothing here: on Linux the mode bits limit what
// an access-control list can give, and Open requires mode 700. On iOS the
// app's own container holds the directory and no other app can reach it, and
// an app cannot start a program such as chmod (MOB-031).
func dropInheritedAccess(string) error { return nil }
