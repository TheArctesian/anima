package unit

import (
	"encoding/binary"
	"os"
	"path/filepath"
	"strings"
	"unicode/utf16"

	"github.com/TheArctesian/anima/link/statedir"
)

// CheckProgram accepts only a protected executable in this user's private directory.
func CheckProgram(program string, _ int) error {
	if !strings.EqualFold(filepath.Ext(program), ".exe") {
		return refused("the Windows program must be an executable")
	}
	folder, err := statedir.Beside(filepath.Dir(program))
	if err != nil {
		return refused("the program folder is not private")
	}
	defer folder.Close()
	if err := folder.CheckFile(filepath.Base(program)); err != nil {
		return refused("the program is not a private regular file")
	}
	return nil
}

func prepareDirectory(path string) error {
	parent := filepath.Dir(path)
	if _, err := os.Stat(parent); os.IsNotExist(err) {
		if err := prepareDirectory(parent); err != nil {
			return err
		}
	}
	folder, err := statedir.Open(path)
	if err != nil {
		return err
	}
	return folder.Close()
}

func commandEnvironment() ([]string, string) {
	names := []string{"SystemRoot", "WINDIR", "USERPROFILE", "LOCALAPPDATA", "APPDATA", "TEMP", "TMP"}
	var env []string
	for _, name := range names {
		if value, ok := os.LookupEnv(name); ok {
			env = append(env, name+"="+value)
		}
	}
	return env, os.Getenv("SystemRoot")
}

// Task Scheduler reads XML as UTF-16. A BOM also preserves non-ASCII profile paths.
func unitBytes(content string) []byte {
	words := utf16.Encode([]rune(content))
	bytes := make([]byte, 2+2*len(words))
	binary.LittleEndian.PutUint16(bytes, 0xfeff)
	for i, word := range words {
		binary.LittleEndian.PutUint16(bytes[2+2*i:], word)
	}
	return bytes
}
