"""Owner-initiated preview updates. No shell execution, data rollback or automatic reboot."""
import fcntl
import io
import json
import lzma
import os
from pathlib import Path
import shutil
import subprocess
import sys
from boot import DIRECTORY, ENTRIES, activate, atomic, pin, select, stage
from feed import FEED, digest, fetch, verify

STATE = DIRECTORY / 'state.json'


def state():
    return json.loads(STATE.read_text()) if STATE.exists() else {'phase': 'idle'}


def save(value):
    atomic(STATE, json.dumps(value)+'\n')


def check():
    stream = io.BytesIO()
    fetch(FEED, stream, 65536)
    release = verify(stream.getvalue())
    previous = state()
    if release['version'] < previous.get('highest', '0'):
        raise ValueError('The server offered an older release. It will not replace this system.')
    current = str(Path('/run/current-system').resolve())
    return {'available': release['system'] != current, 'release': release, 'current': current, 'state': previous}


def download():
    previous = state()
    if previous.get('phase') in ('staged', 'booting'):
        raise ValueError('Finish or roll back the pending update before downloading another.')
    result = check(); release = result['release']
    if not result['available']:
        return {'phase': 'current', 'version': release['version']}
    required = release['bytes'] + release['unpackedBytes'] + 2 * 1024**3
    if shutil.disk_usage('/nix').free < required or shutil.disk_usage('/boot').free < 512*1024**2:
        raise ValueError('Not enough free space to stage the update while retaining recovery.')
    archive = DIRECTORY / 'download.nar.xz'
    try:
        with archive.open('wb') as output:
            count = fetch(release['url'], output, release['bytes'])
        if count != release['bytes'] or digest(archive) != release['sha256']:
            raise ValueError('Downloaded update verification failed; the current system is unchanged.')
        with lzma.open(archive, 'rb') as source:
            process = subprocess.Popen(['nix-store', '--import', '--option', 'require-sigs', 'false'], stdin=subprocess.PIPE,
                                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            try:
                total = 0
                while chunk := source.read(1024*1024):
                    total += len(chunk)
                    if total > release['unpackedBytes']:
                        raise ValueError('The update unpacked beyond its declared limit.')
                    process.stdin.write(chunk)
                process.stdin.close()
                if process.wait(timeout=120):
                    raise ValueError('The verified system could not be imported.')
            finally:
                if process.poll() is None:
                    process.kill(); process.wait()
        system = release['system']
        if not Path(system, 'init').is_file() or not Path(system, 'etc/anima-preview').exists():
            raise ValueError('The imported system is not an Anima appliance.')
        current = stage(system)
        value = {'phase': 'staged', 'version': release['version'], 'highest': release['version'], 'system': system, 'previous': current}
        save(value)
        return value
    finally:
        archive.unlink(missing_ok=True)


def health():
    pending = state()
    if pending.get('phase') != 'booting':
        return pending
    current = str(Path('/run/current-system').resolve())
    if current != pending['system']:
        subprocess.run(['nix-env', '--profile', '/nix/var/nix/profiles/system', '--set', current], check=True, capture_output=True)
        pending['phase'] = 'rolled-back'; save(pending); select('anima-current.conf')
        return pending
    options = subprocess.check_output(['findmnt', '-n', '-o', 'OPTIONS', '/'], text=True).strip().split(',')
    if 'ro' not in options:
        raise ValueError('The system root is not protected.')
    for unit in ('NetworkManager', 'wpa_supplicant', 'display-manager'):
        subprocess.run(['systemctl', 'is-active', '--quiet', unit], check=True)
    if Path('/var/lib/anima-daemon/daemon.json').exists():
        subprocess.run(['systemctl', 'is-active', '--quiet', 'anima-daemon'], check=True)
    subprocess.run(['@blessBoot@', 'good'], check=True)
    subprocess.run(['nix-env', '--profile', '/nix/var/nix/profiles/system', '--set', current], check=True, capture_output=True)
    pin('current', current)
    pending['phase'] = 'current'; save(pending)
    return pending


def dispatch(action):
    if action == 'status':
        return state()
    if action == 'check':
        return check()
    if action == 'download':
        return download()
    if action == 'activate':
        pending = state()
        if pending.get('phase') != 'staged':
            raise ValueError('Download and verify an update first.')
        pending['phase'] = 'booting'; save(pending)
        activate()
        return pending
    if action == 'rollback':
        pending = state()
        if not pending.get('previous') or not (ENTRIES/'anima-current.conf').is_file():
            raise ValueError('There is no previous update to roll back to.')
        subprocess.run(['nix-env', '--profile', '/nix/var/nix/profiles/system', '--set', pending['previous']], check=True, capture_output=True)
        select('anima-current.conf')
        pending['phase'] = 'rolled-back'; save(pending)
        subprocess.run(['systemctl', '--no-block', 'reboot'], check=True)
        return pending
    if action == 'failure':
        pending = state()
        if pending.get('phase') == 'booting' and pending.get('system') == str(Path('/run/current-system').resolve()):
            subprocess.run(['systemctl', '--no-block', 'reboot'], check=True)
        return pending
    if action == 'health':
        return health()
    raise ValueError('Unknown update operation.')


def main():
    if os.geteuid() != 0:
        raise SystemExit('Local owner authentication required.')
    os.umask(0o077); DIRECTORY.mkdir(parents=True, exist_ok=True)
    with open('/run/anima-update.lock', 'w') as lease:
        try:
            fcntl.flock(lease, fcntl.LOCK_EX | fcntl.LOCK_NB)
            result = dispatch(sys.argv[1])
            print(json.dumps({'ok': True, 'value': result}))
        except Exception as error:
            message = str(error) if isinstance(error, ValueError) else 'The update operation failed. Check connectivity and available storage, then retry.'
            print(json.dumps({'ok': False, 'error': message}))
            raise SystemExit(1) from None


if __name__ == '__main__':
    main()
