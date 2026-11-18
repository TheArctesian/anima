//go:build android

package statedir

// foldersAbove does nothing on Android: the directory is in the app's own
// data folder, which only the app's user can reach, and the system's folders
// above it (/data/user/0) are writable by the system's group by design
// (mobile plan M3-1, MOB-031).
func foldersAbove(string) error { return nil }
