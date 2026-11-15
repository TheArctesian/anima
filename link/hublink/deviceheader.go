package hublink

import (
	"encoding/binary"
	"net"
	"net/netip"

	"github.com/TheArctesian/anima/link/tunnel"
)

// The hub sees each stream come from loopback, so it cannot tell devices
// apart. With Config.NameDevice, the hub link puts a PROXY protocol header
// (version 2) at the start of each stream to the hub. The header holds the
// device's address in the link and, in one extra field, the device key that
// the tunnel authenticated.
//
// The hub must read the header only on its loopback port for the link, and
// must not accept it from any other source: whoever can write this header
// names the device. How the hub uses it is plan item S5.

// DeviceKeyField is the type of the PROXY header's extra field that holds
// the device key as text. The protocol keeps 0xE0 to 0xEF for such use.
const DeviceKeyField = 0xE0

var proxySignature = []byte{0x0D, 0x0A, 0x0D, 0x0A, 0x00, 0x0D, 0x0A, 0x51, 0x55, 0x49, 0x54, 0x0A}

const (
	proxyVersion2Proxy = 0x21 // version 2, command PROXY
	proxyTCPOverIPv6   = 0x21 // family IPv6, protocol stream
)

// deviceHeader is the PROXY header for a stream of a device.
func deviceHeader(device tunnel.DeviceKey, from, to netip.AddrPort) []byte {
	keyText, _ := device.MarshalText() // never fails for a key
	addresses := make([]byte, 0, 36)
	source, destination := from.Addr().As16(), to.Addr().As16()
	addresses = append(addresses, source[:]...)
	addresses = append(addresses, destination[:]...)
	addresses = binary.BigEndian.AppendUint16(addresses, from.Port())
	addresses = binary.BigEndian.AppendUint16(addresses, to.Port())

	field := []byte{DeviceKeyField}
	field = binary.BigEndian.AppendUint16(field, uint16(len(keyText)))
	field = append(field, keyText...)

	header := append([]byte{}, proxySignature...)
	header = append(header, proxyVersion2Proxy, proxyTCPOverIPv6)
	header = binary.BigEndian.AppendUint16(header, uint16(len(addresses)+len(field)))
	header = append(header, addresses...)
	return append(header, field...)
}

// addrPortOf is the address and port of one end of a stream.
func addrPortOf(address net.Addr) netip.AddrPort {
	if tcp, ok := address.(*net.TCPAddr); ok {
		return tcp.AddrPort()
	}
	return netip.AddrPort{}
}
