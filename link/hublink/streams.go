package hublink

import (
	"net"
	"sync"

	"github.com/TheArctesian/anima/link/tunnel"
)

// streams records the open streams of each device, so that the hub link can
// close the streams of one device (LNK-004) and count them.
type streams struct {
	mu   sync.Mutex
	open map[tunnel.DeviceKey]map[net.Conn]struct{}
}

func (s *streams) add(device tunnel.DeviceKey, stream net.Conn) {
	s.mu.Lock()
	defer s.mu.Unlock()
	if s.open == nil {
		s.open = map[tunnel.DeviceKey]map[net.Conn]struct{}{}
	}
	if s.open[device] == nil {
		s.open[device] = map[net.Conn]struct{}{}
	}
	s.open[device][stream] = struct{}{}
}

func (s *streams) remove(device tunnel.DeviceKey, stream net.Conn) {
	s.mu.Lock()
	defer s.mu.Unlock()
	delete(s.open[device], stream)
	if len(s.open[device]) == 0 {
		delete(s.open, device)
	}
}

func (s *streams) count(device tunnel.DeviceKey) int {
	s.mu.Lock()
	defer s.mu.Unlock()
	return len(s.open[device])
}
