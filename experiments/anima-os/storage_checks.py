"""Check storage at the actual daemon-to-hub wire boundary inside a disposable VM."""
import base64
from pathlib import Path


def storage_reporting(machine, output):
    script = base64.b64encode(Path(__file__).with_name('storage_probe.py').read_bytes()).decode()
    machine.command('test ! -e /var/lib/anima-daemon/daemon.json')
    machine.command(f"printf '%s' '{script}' | base64 -d > /tmp/anima-storage-probe.py")
    machine.command("python=$(nix-store -qR /run/current-system | grep -E '/[^/]+-python3-[0-9.]+$' | head -n1); systemd-run --unit=anima-storage-fixture --uid=anima-daemon --collect $python/bin/python3 /tmp/anima-storage-probe.py; timeout 15 sh -c 'until test -S /var/lib/anima-daemon/link.sock; do sleep 1; done'")
    try:
        machine.command("printf 'anima_enrol_AAAAAAAAAAAAAAAAAAAAAA\\n' | runuser -u anima-daemon -- env ANIMA_DAEMON_HOME=/var/lib/anima-daemon anima-daemon enrol - --hub unix:/var/lib/anima-daemon/link.sock; systemctl start anima-daemon")
        machine.command("timeout 120 sh -c 'until test -s /var/lib/anima-daemon/fixture-pushes.jsonl && jq -e -s \"any(.[]; (.drives // [] | length) > 0) and any(.[]; any(.series[]; .kind == \\\"disk_percent\\\"))\" /var/lib/anima-daemon/fixture-pushes.jsonl >/dev/null; do sleep 2; done'", timeout=125)
        start = machine.offset
        machine.command("jq -s '[.[] | {drives,series}]' /var/lib/anima-daemon/fixture-pushes.jsonl")
        (output/'storage-wire.txt').write_text(machine.buffer[start:machine.offset])
    finally:
        machine.command('systemctl stop anima-daemon anima-storage-fixture; rm -f /var/lib/anima-daemon/daemon.json /var/lib/anima-daemon/token /var/lib/anima-daemon/link.sock /var/lib/anima-daemon/fixture-pushes.jsonl /tmp/anima-storage-probe.py')
