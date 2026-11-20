package unit

import (
	"encoding/binary"
	"testing"
	"unicode/utf16"
)

func TestWindowsTaskFilePreservesUnicodeWithUTF16BOM(t *testing.T) {
	text := `<?xml version="1.0"?><Command>C:\Users\Zoë\工具.exe</Command>`
	bytes := unitBytes(text)
	if len(bytes)%2 != 0 || binary.LittleEndian.Uint16(bytes) != 0xfeff {
		t.Fatal("Task Scheduler XML must start with a UTF-16LE BOM")
	}
	words := make([]uint16, (len(bytes)-2)/2)
	for i := range words {
		words[i] = binary.LittleEndian.Uint16(bytes[2+2*i:])
	}
	if string(utf16.Decode(words)) != text {
		t.Fatal("task paths changed during encoding")
	}
}
