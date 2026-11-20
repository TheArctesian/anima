"""Disposable serial-console VM harness; no physical disks or host shares."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import pty
import re
import select
import shutil
import subprocess
import sys
import termios
import tty
import time
import uuid


def digest(path):
    checksum = hashlib.sha256()
    with path.open('rb') as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b''):
            checksum.update(chunk)
    return checksum.hexdigest()


class Machine:
    def __init__(self, directory, image, boot_iso=None):
        if any(',' in str(path) or '\n' in str(path) for path in (directory, image)):
            raise ValueError('VM disk paths cannot contain commas or newlines')
        self.directory = directory
        self.buffer = ''
        self.offset = 0
        self.log = (directory / 'serial.log').open('a')
        self.master, slave = pty.openpty()
        args = [
            'qemu-system-x86_64', '-enable-kvm', '-machine', 'q35',
            '-m', '4096', '-smp', '2', '-display', 'none', '-monitor', 'none',
            '-qmp', f'unix:{directory / "qmp.sock"},server=on,wait=off',
            '-serial', 'stdio', '-device', 'i6300esb', '-watchdog-action', 'reset',
            '-drive', 'if=pflash,format=raw,readonly=on,file=/usr/share/edk2/x64/OVMF_CODE.4m.fd',
            '-drive', f'if=pflash,format=raw,file={directory / "vars.fd"}',
            '-drive', f'file={image},format=qcow2,if=virtio',
            '-drive', f'file={directory / "sentinel.raw"},format=raw,if=virtio',
            '-nic', 'none',
        ]
        if boot_iso is not None:
            args.extend(["-cdrom", str(boot_iso), "-boot", "once=d"])
        self.process = subprocess.Popen(args, stdin=slave, stdout=slave, stderr=slave)
        os.close(slave)

    def expect(self, pattern, timeout=180):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            match = re.search(pattern, self.buffer[self.offset:])
            if match:
                self.offset += match.end()
                return match
            if select.select([self.master], [], [], 1)[0]:
                try:
                    chunk = os.read(self.master, 65536).decode(errors='replace')
                except OSError as error:
                    raise RuntimeError('VM console closed') from error
                self.buffer += chunk
                self.log.write(chunk)
                self.log.flush()
            if self.process.poll() is not None:
                raise RuntimeError(f'VM exited {self.process.returncode}')
        raise TimeoutError(f'No {pattern!r} after {timeout}s; see serial.log')

    def send(self, command):
        os.write(self.master, (command + '\n').encode())

    def ready(self, revision='A', timeout=180):
        try:
            self.expect(f'ANIMA_READY revision={revision}', timeout=timeout)
        except TimeoutError as failure:
            self.send("stty -echo; journalctl -b -u anima-root-lock -u anima-local-health --no-pager; findmnt /; printf '\\nANIMA_%s\\n' DIAGNOSTICS_END")
            try:
                self.expect('ANIMA_DIAGNOSTICS_END', timeout=5)
            except (TimeoutError, RuntimeError):
                pass  # A stuck initrd may have no shell; preserve the readiness failure.
            raise failure
        self.expect(r'root@anima-prototype:[^\r\n]*#')
        self.send('export PS1="ANIMA_PROMPT> "; stty -echo')
        self.expect('ANIMA_PROMPT> ')

    def command(self, command, timeout=30):
        marker = f'ANIMA_DONE_{time.monotonic_ns()}'
        self.send(f"( set -e -o pipefail; {command} ); printf '\\n{marker}:%s\\n' \"$?\"")
        result = self.expect(re.escape(marker) + r':(\d+)', timeout)
        if result.group(1) != '0':
            raise RuntimeError(f'Guest command failed: {command}')

    def reboot(self, revision='A'):
        self.send('systemctl reboot')
        self.ready(revision)

    def wait_exit(self, timeout=90):
        # Continue draining serial output: a full PTY can block QEMU shutdown.
        deadline = time.monotonic() + timeout
        while self.process.poll() is None:
            if time.monotonic() >= deadline:
                raise TimeoutError('VM did not exit after shutdown')
            if select.select([self.master], [], [], 1)[0]:
                try:
                    chunk = os.read(self.master, 65536).decode(errors='replace')
                except OSError:
                    continue
                self.buffer += chunk
                self.log.write(chunk)
                self.log.flush()
        if self.process.returncode != 0:
            raise RuntimeError(f'VM exited {self.process.returncode}')

    def close(self):
        if self.log.closed:
            return
        if self.process.poll() is None:
            self.process.terminate()
            try:
                self.process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                self.process.kill()
                self.process.wait()
        os.close(self.master)
        self.log.close()


def prepare(image, output):
    # Refuse reuse: every run owns only a fresh directory and regular image copy.
    if not image.is_file() or image.is_symlink():
        raise ValueError('Pass a regular resolved qcow2 file')
    output.mkdir(parents=True, exist_ok=False)
    filesystem = subprocess.check_output(['stat', '-f', '-c', '%T', str(output)], text=True).strip()
    if filesystem in ('tmpfs', 'ramfs') or shutil.disk_usage(output).free < 32 * 1024**3:
        raise ValueError('VM tests need a disk-backed output directory with at least 32 GiB free')
    (output / 'manifest.json').write_text(json.dumps({
        'image': str(image), 'sha256': digest(image),
        'qemu': subprocess.check_output(['qemu-system-x86_64', '--version'], text=True).splitlines()[0],
        'guest': {'cpus': 2, 'memory_mib': 4096, 'network': 'none'},
        'started_unix': time.time(),
    }, indent=2) + '\n')
    disk = output / 'anima.qcow2'
    subprocess.run(['cp', '--reflink=auto', '--sparse=always', str(image), str(disk)], check=True)
    disk.chmod(0o600)
    shutil.copy('/usr/share/edk2/x64/OVMF_VARS.4m.fd', output / 'vars.fd')
    (output / 'vars.fd').chmod(0o600)
    sentinel = output / 'sentinel.raw'
    with sentinel.open('wb') as stream:
        stream.write(b'ANIMA_UNSELECTED_DISK_SENTINEL')
        stream.truncate(1024 ** 3)
    before = digest(sentinel)
    return disk, before


def run(image, output):
    disk, before = prepare(image, output)
    sentinel = output / "sentinel.raw"
    results = []
    machine = Machine(output, disk)
    started = time.monotonic()
    try:
        machine.ready()
        results.append({'case': 'uefi-boot', 'result': 'Passed', 'seconds': time.monotonic()-started})
        machine.command('findmnt -n -o FSTYPE / | grep -qx btrfs')
        machine.command("findmnt -n -o OPTIONS / | tr ',' '\\n' | grep -qx ro")
        nonce = uuid.uuid4().hex
        machine.command(f'printf %s {nonce}-identity > /persist/identity/sentinel; printf %s {nonce}-work > /persist/workload/sentinel')
        machine.command('sha256sum /persist/identity/sentinel /persist/workload/sentinel > /var/lib/anima-prototype/sentinels.sha256')
        for principal in ['workload', 'daemon-test']:
            for target in ['/write-test', '/nix/store/write-test', '/boot/write-test']:
                machine.command(f'! runuser -u {principal} -- touch {target}')
        results.append({'case': 'btrfs-read-only-and-principals', 'result': 'Passed'})
        for number in range(5):
            machine.reboot()
            machine.command('sha256sum -c /var/lib/anima-prototype/sentinels.sha256')
            results.append({'case': f'persistent-reboot-{number+1}', 'result': 'Passed'})
        for number in range(3):
            machine.command("rm -f /persist/fail-health; mv /boot/loader/entries/anima-b.conf /boot/loader/entries/anima-b+2.conf; printf 'default anima-*.conf\\ntimeout 1\\n' > /boot/loader/loader.conf; sync")
            machine.reboot('B')
            machine.command('test -f /boot/loader/entries/anima-b.conf && hello && systemctl is-active anima-custom-test && sha256sum -c /var/lib/anima-prototype/sentinels.sha256')
            results.append({'case': f'healthy-candidate-{number+1}', 'result': 'Passed'})
            machine.command("printf 'default anima-a.conf\\ntimeout 1\\n' > /boot/loader/loader.conf; sync")
            machine.reboot('A')
            machine.command('! command -v hello && test -f /persist/workload/service-output')
        for number in range(3):
            machine.command("touch /persist/fail-health; mv /boot/loader/entries/anima-b.conf /boot/loader/entries/anima-b+2.conf; printf 'default anima-*.conf\\ntimeout 1\\n' > /boot/loader/loader.conf; sync")
            machine.send('systemctl reboot')
            machine.expect('ANIMA_HEALTH_FAILED')
            machine.expect('ANIMA_HEALTH_FAILED')
            machine.ready('A')
            machine.command('sha256sum -c /var/lib/anima-prototype/sentinels.sha256')
            results.append({'case': f'health-fallback-{number+1}', 'result': 'Passed'})
            machine.command("rm /persist/fail-health; mv /boot/loader/entries/anima-b+0-2.conf /boot/loader/entries/anima-b.conf; printf 'default anima-a.conf\\ntimeout 1\\n' > /boot/loader/loader.conf; sync")
        machine.send('systemctl poweroff')
        machine.wait_exit(timeout=60)
    except Exception as error:
        results.append({'case': 'run', 'result': 'Failed', 'reason': str(error)})
        raise
    finally:
        machine.close()
        after = digest(sentinel)
        results.append({'case': 'unselected-disk', 'result': 'Passed' if before == after else 'Failed', 'before': before, 'after': after})
        (output / 'results.json').write_text(json.dumps(results, indent=2) + '\n')
    if any(result['result'] != 'Passed' for result in results):
        raise RuntimeError('VM checks failed; see results.json')


def interactive(image, output):
    if not sys.stdin.isatty():
        raise ValueError('Interactive mode requires a terminal')
    disk, _ = prepare(image, output)
    machine = Machine(output, disk)
    previous = termios.tcgetattr(sys.stdin)
    print("Disposable Anima console; Ctrl-] exits and stops this VM.", flush=True)
    try:
        tty.setraw(sys.stdin.fileno())
        while machine.process.poll() is None:
            ready, _, _ = select.select([sys.stdin.fileno(), machine.master], [], [], 1)
            for source in ready:
                chunk = os.read(source, 65536)
                if not chunk or (source == sys.stdin.fileno() and b'\x1d' in chunk):
                    return
                os.write(machine.master if source == sys.stdin.fileno() else sys.stdout.fileno(), chunk)
    finally:
        termios.tcsetattr(sys.stdin, termios.TCSADRAIN, previous)
        machine.close()


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('image', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--interactive', action='store_true')
    args = parser.parse_args()
    action = interactive if args.interactive else run
    action(args.image.resolve(), args.output.absolute())
