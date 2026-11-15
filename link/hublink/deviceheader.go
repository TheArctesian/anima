package hublink

import (
	"encoding/binary"
	"net"
	"net/netip"
)

// The hub sees each stream come from the link, so it cannot tell devices
// apart. When the target is the hub's socket, the hub link therefore puts a
// PROXY protocol header (version 2) at the start of each stream. The
// header's source address is the link address of the device whose key the
// tunnel authenticated (see devices/address.go).
//
// Whoever can write this header names the device. The hub reads it only on
// its socket for the link, where no other user's program can connect. A
// loopback port has no such limit, so a loopback target gets no header.

var proxySignature = []byte{0x0D, 0x0A, 0x0D, 0x0A, 0x00, 0x0D, 0x0A, 0x51, 0x55, 0x49, 0x54, 0x0A}

const (
	proxyVersion2Proxy = 0x21 // version 2, command PROXY
	proxyTCPOverIPv6   = 0x21 // family IPv6, protocol stream
	proxyIPv6Addresses = 36   // two addresses and two ports
)

// deviceHeader is the PROXY header for a stream of the device that has a
// link address. The destination is the hub's end of the stream in the
// tunnel; the source port is the device's port of the stream.
func deviceHeader(device netip.Addr, sourcePort uint16, to netip.AddrPort) []byte {
	header := append([]byte{}, proxySignature...)
	header = append(header, proxyVersion2Proxy, proxyTCPOverIPv6)
	header = binary.BigEndian.AppendUint16(header, proxyIPv6Addresses)
	source, destination := device.As16(), to.Addr().As16()
	header = append(header, source[:]...)
	header = append(header, destination[:]...)
	header = binary.BigEndian.AppendUint16(header, sourcePort)
	return binary.BigEndian.AppendUint16(header, to.Port())
}

// addrPortOf is the address and port of one end of a stream.
func addrPortOf(address net.Addr) netip.AddrPort {
	if tcp, ok := address.(*net.TCPAddr); ok {
		return tcp.AddrPort()
	}
	return netip.AddrPort{}
}
