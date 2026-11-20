"""Exercise Calamares through its real X11 UI on disposable, offline VM disks."""
import argparse
import json
from pathlib import Path
import secrets
import shutil
import socket
import subprocess
import time
from vm import Machine, digest


def qmp(directory, command, arguments):
    with socket.socket(socket.AF_UNIX) as connection:
        connection.connect(str(directory / 'qmp.sock'))
        with connection.makefile('rwb', buffering=0) as stream:
            stream.readline()
            for request in ({'execute': 'qmp_capabilities'}, {'execute': command, 'arguments': arguments}):
                stream.write((json.dumps(request)+'\n').encode())
                while True:
                    response = json.loads(stream.readline())
                    if 'error' in response:
                        raise RuntimeError(response['error'])
                    if 'return' in response:
                        break


def screenshot(directory, name):
    qmp(directory, 'screendump', {'filename': str(directory / f'{name}.ppm')})


def click(machine, x, y):
    machine.command(f'DISPLAY=:0 XAUTHORITY=/home/anima/.Xauthority xdotool mousemove {x} {y} click 1')
    time.sleep(0.5)


def enter_password(machine, output, password):
    for x in (400, 600):
        click(machine, x, 300)
        for character in password:
            qmp(output, 'send-key', {'keys': [{'type': 'qcode', 'data': character}], 'hold-time': 20})
            time.sleep(0.04)


def prepare(output):
    output.mkdir(parents=True, exist_ok=False)
    if subprocess.check_output(['stat', '-f', '-c', '%T', str(output)], text=True).strip() in ('tmpfs', 'ramfs'):
        raise ValueError('Use disk-backed storage')
    if shutil.disk_usage(output).free < 64 * 1024**3:
        raise ValueError('At least 64 GiB free required')
    disk = output / 'installed.qcow2'
    subprocess.run(['qemu-img', 'create', '-f', 'qcow2', str(disk), '64G'], check=True)
    shutil.copy('/usr/share/edk2/x64/OVMF_VARS.4m.fd', output / 'vars.fd')
    with (output / 'sentinel.raw').open('wb') as stream:
        stream.write(b'ANIMA_GUI_UNSELECTED_DISK')
        stream.truncate(32 * 1024**3)
    return disk


def graphical_install(machine, output, password):
    machine.expect(r'root@anima-live:[^\r\n]*#', timeout=240)
    machine.send('stty -echo')
    machine.command("timeout 90 sh -c 'until grep -q \"2 devices detected\" /root/.cache/calamares/session.log 2>/dev/null; do sleep 1; done'", timeout=100)
    time.sleep(5)
    screenshot(output, 'welcome')
    machine.command('head -c 1048576 /dev/vda | sha256sum > /run/before-disk')
    machine.command('nmcli connection add type ethernet con-name anima-fixture ifname fixture0; nmcli connection modify anima-fixture connection.autoconnect no')
    click(machine, 987, 749)  # Welcome -> keyboard
    click(machine, 987, 749)  # Keyboard -> owner
    enter_password(machine, output, password)
    click(machine, 987, 749)  # Owner -> disks
    screenshot(output, 'disks')
    click(machine, 325, 190)  # Explicitly choose erase on the first, 64 GiB disk.
    click(machine, 987, 749)
    screenshot(output, 'summary')
    click(machine, 987, 749)
    screenshot(output, 'confirmation')
    click(machine, 692, 444)  # Go back from destructive confirmation.
    machine.command('head -c 1048576 /dev/vda | sha256sum | diff /run/before-disk -')
    click(machine, 987, 749)
    click(machine, 815, 444)
    machine.command("timeout 1200 sh -c 'until grep -q ANIMA_INSTALL_COMPLETE /root/.cache/calamares/session.log; do if grep -q \"Installation failed:\" /root/.cache/calamares/session.log; then exit 1; fi; sleep 2; done'", timeout=1210)
    time.sleep(2)
    screenshot(output, 'finished')


def installed_checks(machine, password):
    machine.expect('anima login:', timeout=240)
    machine.send('admin'); machine.expect('Password:'); machine.send(password)
    machine.expect(r'admin@anima:[^\r\n]*\$'); machine.send('stty -echo')
    machine.command("findmnt -n -o OPTIONS / | tr ',' '\\n' | grep -qx ro; test -w /home/admin; ! touch /host-write-test")
    machine.command('command -v lspci; command -v nmcli; systemctl is-active NetworkManager; systemctl show -p Result --value anima-keymap | grep -qx success; systemctl show -p ExecMainStartTimestampMonotonic --value anima-keymap | grep -qv "^0$"; test -s /persist/console.map')
    machine.command('nmcli -g NAME connection show | grep -qx anima-fixture')
    machine.command("podman info --format '{{.Host.Security.Rootless}}' | grep -qx true", timeout=60)
    machine.send('sudo -v'); machine.expect('password for admin:'); machine.send(password)
    machine.expect(r'admin@anima:[^\r\n]*\$')
    machine.command('sudo test -r /persist/owner-password; sudo find /persist/network-connections -name "*.nmconnection" -printf "%m\\n" | grep -qx 600')
    machine.command('echo retained > ~/gui-sentinel; cp /etc/machine-id ~/machine-id-before')
    machine.send('sudo systemctl reboot')
    machine.expect('anima login:', timeout=240)
    machine.send('admin'); machine.expect('Password:'); machine.send(password)
    machine.expect(r'admin@anima:[^\r\n]*\$'); machine.send('stty -echo')
    machine.command('grep -qx retained ~/gui-sentinel; cmp /etc/machine-id ~/machine-id-before; nmcli -g NAME connection show | grep -qx anima-fixture')


def run(iso, output):
    disk = prepare(output)
    before = digest(output / 'sentinel.raw')
    password = secrets.token_hex(12)
    results = []
    machine = Machine(output, disk, boot_iso=iso)
    try:
        graphical_install(machine, output, password)
        results.append({'case': 'graphical-offline-install-and-cancel', 'result': 'Passed'})
        machine.send('systemctl poweroff'); machine.wait_exit(); machine.close()
        machine = Machine(output, disk)
        installed_checks(machine, password)
        screenshot(output, 'installed-console')
        results.append({'case': 'installed-login-readonly-network-keyboard-persistence', 'result': 'Passed'})
    except Exception as error:
        results.append({'case': 'gui-run', 'result': 'Failed', 'reason': str(error)})
        screenshot(output, 'failure')
    finally:
        machine.close()
        after = digest(output / 'sentinel.raw')
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
