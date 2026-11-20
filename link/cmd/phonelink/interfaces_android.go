package main

/*
#include <ifaddrs.h>
#include <net/if.h>
#include <netinet/in.h>

static const void* ip_bytes(const struct sockaddr* address) {
	if (address == NULL) {
		return NULL;
	}
	switch (address->sa_family) {
	case AF_INET:
		return &((const struct sockaddr_in*)address)->sin_addr;
	case AF_INET6:
		return &((const struct sockaddr_in6*)address)->sin6_addr;
	default:
		return NULL;
	}
}
*/
import "C"

import (
	"fmt"
	"net"

	"tailscale.com/net/netmon"
)

func init() {
	// Android's libc enumerates interfaces without the netlink operation denied to app UIDs.
	netmon.RegisterInterfaceGetter(androidInterfaces)
}

func androidInterfaces() ([]netmon.Interface, error) {
	var first *C.struct_ifaddrs
	if result, err := C.getifaddrs(&first); result != 0 {
		return nil, fmt.Errorf("get Android interfaces: %w", err)
	}
	defer C.freeifaddrs(first)
	var interfaces []netmon.Interface
	byName := make(map[string]int)
	for address := first; address != nil; address = address.ifa_next {
		ip := interfaceAddress(address)
		if ip == nil {
			continue
		}
		name := C.GoString(address.ifa_name)
		index, exists := byName[name]
		if !exists {
			index = len(interfaces)
			byName[name] = index
			interfaces = append(interfaces, netmon.Interface{
				Interface: &net.Interface{
					Name:  name,
					Index: int(C.if_nametoindex(address.ifa_name)),
					Flags: interfaceFlags(address.ifa_flags),
				},
			})
		}
		interfaces[index].AltAddrs = append(interfaces[index].AltAddrs, ip)
	}
	return interfaces, nil
}

func interfaceAddress(address *C.struct_ifaddrs) *net.IPNet {
	ip := C.ip_bytes(address.ifa_addr)
	mask := C.ip_bytes(address.ifa_netmask)
	if ip == nil || mask == nil || address.ifa_addr.sa_family != address.ifa_netmask.sa_family {
		return nil
	}
	size := C.int(net.IPv4len)
	if address.ifa_addr.sa_family == C.AF_INET6 {
		size = net.IPv6len
	}
	return &net.IPNet{IP: C.GoBytes(ip, size), Mask: C.GoBytes(mask, size)}
}

func interfaceFlags(flags C.uint) net.Flags {
	var result net.Flags
	for _, flag := range [...]struct {
		native C.uint
		goFlag net.Flags
	}{
		{C.IFF_UP, net.FlagUp},
		{C.IFF_RUNNING, net.FlagRunning},
		{C.IFF_LOOPBACK, net.FlagLoopback},
		{C.IFF_BROADCAST, net.FlagBroadcast},
		{C.IFF_POINTOPOINT, net.FlagPointToPoint},
		{C.IFF_MULTICAST, net.FlagMulticast},
	} {
		if flags&flag.native != 0 {
			result |= flag.goFlag
		}
	}
	return result
}
