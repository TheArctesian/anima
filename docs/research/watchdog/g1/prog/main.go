// Command g1prog is the watched program of the G1 prototype (watchdog.md, task G1). It is not
// part of Anima. Modes:
//
//	g1prog serve [unhealthy-after-seconds]  runs until SIGTERM, then exits 0; writes the file
//	                                        "ok" in its working folder and deletes it after the
//	                                        given seconds (0: never)
//	g1prog crash <seconds> <code>           exits with <code> after <seconds>
//	g1prog health                           exits 0 when the file "ok" exists, else 1
package main

import (
	"fmt"
	"os"
	"os/signal"
	"strconv"
	"syscall"
	"time"
)

func main() {
	if len(os.Args) < 2 {
		fmt.Fprintln(os.Stderr, "usage: g1prog serve|crash|health")
		os.Exit(2)
	}
	switch os.Args[1] {
	case "serve":
		serve(argument(2, 0))
	case "crash":
		time.Sleep(time.Duration(argument(2, 1)) * time.Second)
		os.Exit(argument(3, 1))
	case "health":
		if _, err := os.Stat("ok"); err != nil {
			os.Exit(1)
		}
	default:
		os.Exit(2)
	}
}

func serve(unhealthyAfter int) {
	if err := os.WriteFile("ok", []byte("ok"), 0o600); err != nil {
		fmt.Fprintln(os.Stderr, err)
	}
	if unhealthyAfter > 0 {
		time.AfterFunc(time.Duration(unhealthyAfter)*time.Second, func() { _ = os.Remove("ok") })
	}
	stop := make(chan os.Signal, 1)
	signal.Notify(stop, syscall.SIGTERM, syscall.SIGINT)
	got := <-stop
	fmt.Println("stopped by", got)
}

func argument(index, fallback int) int {
	if len(os.Args) <= index {
		return fallback
	}
	value, err := strconv.Atoi(os.Args[index])
	if err != nil {
		return fallback
	}
	return value
}
