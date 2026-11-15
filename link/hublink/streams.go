package hublink

import (
	"net"
	"sync"

	"github.com/TheArctesian/anima/link/tunnel"
)

// streams records the open streams of each device, so that the hub link can
// close the streams of one device (LNK-004), close all of them when it
// stops, and count them.
type streams struct {
	mu     sync.Mutex
	open   map[tunnel.DeviceKey]map[net.Conn]struct{}
	closed bool
	active sync.WaitGroup
}

// add records a stream. It returns false when the hub link has stopped: the
// caller then closes the stream.
func (s *streams) add(device tunnel.DeviceKey, stream net.Conn) bool {
	s.mu.Lock()
	defer s.mu.Unlock()
	if s.closed {
		return false
	}
	if s.open == nil {
		s.open = map[tunnel.DeviceKey]map[net.Conn]struct{}{}
	}
	if s.open[device] == nil {
		s.open[device] = map[net.Conn]struct{}{}
	}
	s.open[device][stream] = struct{}{}
	s.active.Add(1)
	return true
}

func (s *streams) remove(device tunnel.DeviceKey, stream net.Conn) {
	s.mu.Lock()
	defer s.mu.Unlock()
	delete(s.open[device], stream)
	if len(s.open[device]) == 0 {
		delete(s.open, device)
	}
	s.active.Done()
}

func (s *streams) count(device tunnel.DeviceKey) int {
	s.mu.Lock()
	defer s.mu.Unlock()
	return len(s.open[device])
}

// closeAll closes every stream, takes no new one, and waits until each
// stream's copy to the hub has ended.
func (s *streams) closeAll() {
	s.mu.Lock()
	s.closed = true
	for _, ofDevice := range s.open {
		for stream := range ofDevice {
			stream.Close()
		}
	}
	s.mu.Unlock()
	s.active.Wait()
}
