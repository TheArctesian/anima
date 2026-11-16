//go:build !darwin

package statedir

// dropInheritedAccess does nothing here: on Linux the mode bits limit what
// an access-control list can give, and Open requires mode 700.
func dropInheritedAccess(string) error { return nil }
