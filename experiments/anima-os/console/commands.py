"""Bounded local commands. Command output containing credentials never reaches the UI."""
import json
import os
import shutil
import subprocess


def run(args, secret=None, timeout=25):
    result = subprocess.run(args, input=secret, capture_output=True, text=True, timeout=timeout)
    if result.returncode:
        raise ValueError('The operation failed. Check connectivity and try again.')
    return result.stdout.strip()


def fields(line):
    values, word, escaped = [], '', False
    for char in line:
        if escaped:
            word += char
            escaped = False
        elif char == '\\':
            escaped = True
        elif char == ':':
            values.append(word)
            word = ''
        else:
            word += char
    return values + [word]


def devices():
    return [dict(zip(('device', 'type', 'state', 'connection'), fields(line)))
            for line in run(['nmcli', '-t', '-f', 'DEVICE,TYPE,STATE,CONNECTION', 'device']).splitlines()]


def networks():
    rows = run(['nmcli', '-t', '-f', 'DEVICE,BSSID,SSID,SIGNAL,SECURITY', 'device', 'wifi', 'list', '--rescan', 'yes'])
    return [dict(zip(('device', 'bssid', 'ssid', 'signal', 'security'), fields(line))) for line in rows.splitlines()]


def connect(args):
    target = next((item for item in networks() if item['device'] == args['device'] and item['bssid'] == args['bssid']), None)
    if target is None:
        raise ValueError('That network is no longer available. Scan again.')
    password = args.get('password', '')
    if '\n' in password or '\r' in password or len(password) > 256:
        raise ValueError('Invalid network password.')
    run(['nmcli', '--wait', '35', '--ask', 'device', 'wifi', 'connect', target['bssid'], 'ifname', target['device']], password+'\n', 40)
    return status()


def status():
    disk = shutil.disk_usage('/persist')
    service = subprocess.run(['systemctl', 'is-active', 'anima-daemon'], capture_output=True, text=True).stdout.strip()
    drives = json.loads(run(['lsblk', '-J', '-b', '-o', 'NAME,MODEL,SIZE,TYPE,MOUNTPOINTS']))['blockdevices']
    return {'hostname': os.uname().nodename, 'devices': devices(), 'storage': {'used': disk.used, 'total': disk.total},
            'drives': drives, 'enrolled': os.path.exists('/var/lib/anima-daemon/daemon.json'), 'daemon': service,
            'network': run(['nmcli', '-t', '-f', 'STATE,CONNECTIVITY', 'general'])}
