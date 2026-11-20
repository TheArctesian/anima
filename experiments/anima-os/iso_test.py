"""Boot an actual ISO, cancel safely, install offline, then boot its installed disk."""
import argparse
import json
from pathlib import Path
import secrets
import shutil
import subprocess
import time
from vm import Machine, digest


def run(iso, output):
    if not iso.is_file():
        raise ValueError('ISO file is missing')
    for command in ('qemu-img', 'qemu-system-x86_64'):
        if shutil.which(command) is None:
            raise ValueError(f'{command} must be on PATH')
    output.mkdir(parents=True, exist_ok=False)
    if subprocess.check_output(['stat', '-f', '-c', '%T', str(output)], text=True).strip() in ('tmpfs', 'ramfs'):
        raise ValueError('Use disk-backed storage for ISO tests')
    if shutil.disk_usage(output).free < 64*1024**3:
        raise ValueError('At least 64 GiB free required')
    disk = output / 'installed.qcow2'
    subprocess.run(['qemu-img', 'create', '-f', 'qcow2', str(disk), '64G'], check=True)
    shutil.copy('/usr/share/edk2/x64/OVMF_VARS.4m.fd', output / 'vars.fd')
    sentinel = output / 'sentinel.raw'
    with sentinel.open('wb') as stream:
        stream.write(b'ANIMA_ISO_UNSELECTED_DISK')
        stream.truncate(32*1024**3)
    before = digest(sentinel)
    results = []
    password = secrets.token_hex(12)  # Disposable VM-only account, never a production credential.
    machine = Machine(output, disk, boot_iso=iso)
    try:
        machine.expect(r'root@anima-live:[^\r\n]*#', timeout=240)
        machine.send('stty -echo')
        machine.command('busctl call org.freedesktop.DBus /org/freedesktop/DBus org.freedesktop.DBus StartServiceByName su fi.w1.wpa_supplicant1 0; busctl get-property fi.w1.wpa_supplicant1 /fi/w1/wpa_supplicant1 fi.w1.wpa_supplicant1 Interfaces')
        results.append({'case': 'wifi-supplicant-dbus-activation', 'result': 'Passed'})
        machine.command('systemctl stop display-manager; pkill -TERM -x calamares || true; sleep 1')
        machine.command('test -d /sys/firmware/efi; test ! -e /var/lib/anima-daemon/token')
        machine.command('parted -s /dev/vda mklabel gpt mkpart primary ext4 1MiB 65MiB; udevadm settle; mkfs.ext4 -F /dev/vda1; mkdir /mnt/blocked-target; mount -t ext4 /dev/vda1 /mnt/blocked-target', timeout=60)
        start = machine.offset
        machine.send('anima-install'); machine.expect('Disk number')
        offered = machine.buffer[start:machine.offset]
        if '/dev/vda' in offered or '/dev/vdb' not in offered:
            raise RuntimeError('Mounted target was offered or unrelated eligible disk was missing')
        machine.send(''); machine.expect('Cancelled; no disk changed.')
        machine.command('umount /mnt/blocked-target; wipefs --all /dev/vda; udevadm settle')
        results.append({'case': 'mounted-child-excluded', 'result': 'Passed'})
        machine.command('head -c 1048576 /dev/vda | sha256sum > /run/before-disk')
        results.append({'case': 'iso-uefi-live', 'result': 'Passed'})
        for confirm in (False, True):
            machine.send('anima-install')
            machine.expect('Disk number'); machine.send('1')
            machine.expect('Password for the local admin account'); machine.send(password)
            machine.expect('Repeat password:'); machine.send(password)
            machine.expect('type exactly ERASE /dev/vda:')
            machine.send('ERASE /dev/vda' if confirm else 'CANCEL')
            if not confirm:
                machine.expect('Cancelled; no disk changed.')
                machine.command('head -c 1048576 /dev/vda | sha256sum | diff /run/before-disk -')
                results.append({'case': 'cancel-before-erase', 'result': 'Passed'})
            else:
                finish = machine.expect(r'ANIMA_INSTALL_COMPLETE|Installation stopped:|Traceback \(most recent call last\):', timeout=1200)
                if finish.group(0) != 'ANIMA_INSTALL_COMPLETE':
                    raise RuntimeError('Installer reported failure; see serial.log')
                results.append({'case': 'offline-install', 'result': 'Passed'})
        machine.send('systemctl poweroff')
        machine.wait_exit(timeout=90)
        machine.close()
        machine = Machine(output, disk)  # ISO removed: this must boot the installed disk.
        machine.expect('anima login:', timeout=240)
        machine.send('admin')
        machine.expect('Password:'); machine.send(password)
        machine.expect(r'admin@anima:[^\r\n]*\$')
        machine.send('stty -echo')
        machine.command("findmnt -n -o OPTIONS / | tr ',' '\\n' | grep -qx ro; test -w /home/admin; ! touch /host-write-test")
        machine.command('echo retained > ~/iso-sentinel; cp /etc/machine-id ~/machine-id-before; command -v anima-daemon; command -v podman; ! systemctl is-active anima-daemon')
        machine.command('if anima-daemon >/tmp/usage 2>&1; then exit 1; else test "$?" = 2; fi; grep -q usage /tmp/usage')
        results.append({'case': 'installed-login-readonly-runtime', 'result': 'Passed'})
        machine.command("podman info --format '{{.Host.Security.Rootless}}' | grep -qx true", timeout=60)
        machine.command('podman load -i /etc/anima-smoke-image.tar.gz', timeout=120)
        limits = '--pull=never --network=none --read-only --memory=64m --cpus=1 --cap-drop=ALL --security-opt=no-new-privileges'
        machine.command(f'podman run -d --name anima-smoke {limits} anima-preview-smoke:local /bin/httpd -f -p 8080 -h /www', timeout=60)
        machine.command('podman exec anima-smoke /bin/wget -qO- http://127.0.0.1:8080 | grep -qx anima-app-ok; podman rm -f anima-smoke', timeout=60)
        machine.command(f'podman run --rm {limits} anima-preview-smoke:local /bin/sh -c "! touch /write-test && echo anima-job-ok" | grep -qx anima-job-ok', timeout=60)
        results.append({'case': 'rootless-isolated-app-and-job', 'result': 'Passed', 'limitation': 'Local Podman only, not Anima orchestration or an agent run'})
        machine.send('sudo systemctl reboot'); machine.expect(r'password for admin:'); machine.send(password)
        machine.expect('anima login:', timeout=240); machine.send('admin')
        machine.expect('Password:'); machine.send(password)
        machine.expect(r'admin@anima:[^\r\n]*\$'); machine.send('stty -echo')
        machine.command('grep -qx retained ~/iso-sentinel; cmp /etc/machine-id ~/machine-id-before')
        results.append({'case': 'installed-persistence', 'result': 'Passed'})
    except Exception as error:
        results.append({'case': 'iso-run', 'result': 'Failed', 'reason': str(error)})
    finally:
        machine.close()
        after = digest(sentinel)
        results.append({'case': 'unselected-disk', 'result': 'Passed' if before == after else 'Failed'})
        (output / 'results.json').write_text(json.dumps({'iso_sha256': digest(iso), 'results': results}, indent=2)+'\n')
    if any(result['result'] != 'Passed' for result in results):
        raise SystemExit(1)


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('iso', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    run(args.iso.resolve(), args.output.resolve())
