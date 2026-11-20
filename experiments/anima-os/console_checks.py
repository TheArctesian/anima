"""Installed-console acceptance through rendered controls, using disposable VM credentials."""
import time
from gui_test import qmp, screenshot
from gui_actions import locate


def key(output, code):
    qmp(output, 'send-key', {'keys': [{'type': 'qcode', 'data': code}], 'hold-time': 20})
    time.sleep(.04)


def click(machine, x, y):
    machine.command(f'DISPLAY=:0 XAUTHORITY=/home/admin/.Xauthority xdotool mousemove {x} {y} click 1')
    time.sleep(.3)


def text(output, value):
    for char in value:
        key(output, char)


def wait_label(output, phrase, timeout=60):
    deadline = time.monotonic()+timeout
    while time.monotonic() < deadline:
        try:
            return locate(output, phrase)
        except ValueError:
            time.sleep(1)
    raise RuntimeError('Installed UI did not show: '+phrase)


def installed_console(machine, output, password):
    wait_label(output, 'Local session locked')
    screenshot(output, 'console-locked')
    text(output, 'wrong'); key(output, 'ret')
    wait_label(output, 'Authentication failed')
    text(output, password); key(output, 'ret')
    wait_label(output, 'Your Anima server')
    wait_label(output, 'Anima OS Preview')
    screenshot(output, 'console-overview')
    x, y, w, h = locate(output, 'Network'); click(machine, x+w//2, y+h//2)
    wait_label(output, 'Scan networks')
    screenshot(output, 'console-network')
    x, y, w, h = locate(output, 'Display'); click(machine, x+w//2, y+h//2)
    wait_label(output, 'Orientation')
    screenshot(output, 'console-display')
    # The lock control sits at the right of the fullscreen header.
    import subprocess
    screenshot(output, 'current')
    width = int(subprocess.check_output(['magick', 'identify', '-format', '%w', str(output/'current.ppm')], text=True))
    click(machine, width-67, 52)
    wait_label(output, 'Local session locked')
    machine.command('systemctl is-active NetworkManager; systemctl is-active display-manager')
    screenshot(output, 'console-relocked')


def installed_wifi(machine, password):
    machine.send('sudo -i'); machine.expect(r'password for admin:'); machine.send(password)
    machine.expect(r'root@anima:[^\r\n]*#')
    try:
        machine.command("findmnt -n -o OPTIONS / | tr ',' '\\n' | grep -qx ro; systemctl restart wpa_supplicant || { journalctl -u wpa_supplicant --no-pager -n 25; exit 1; }; busctl get-property fi.w1.wpa_supplicant1 /fi/w1/wpa_supplicant1 fi.w1.wpa_supplicant1 Interfaces", timeout=60)
    finally:
        machine.send('exit'); machine.expect(r'admin@anima:[^\r\n]*\$')


def installed_orientation(machine):
    machine.command("test -r /persist/display.json; timeout 90 sh -c 'until DISPLAY=:0 XAUTHORITY=/home/admin/.Xauthority anima-display --list >/dev/null 2>&1; do sleep 1; done'; test \"$(DISPLAY=:0 XAUTHORITY=/home/admin/.Xauthority anima-display --list)\" = \"$(jq -r 'to_entries[] | [.key,.value] | @tsv' /persist/display.json)\"", timeout=100)
