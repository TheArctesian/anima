"""Build a signed update delivery directory from a completed local Nix system closure."""
import argparse
import base64
from datetime import datetime, timezone
import json
from pathlib import Path
import subprocess
from feed import FEED, digest


def build(system, destination, key):
    version = datetime.now(timezone.utc).strftime('%Y%m%d%H%M%S')
    folder = destination / version
    folder.mkdir(parents=True, exist_ok=False)
    paths = subprocess.check_output(['nix-store', '--query', '--requisites', str(system)], text=True).splitlines()
    archive = folder / 'system.nar.xz'
    process = subprocess.Popen(['nix-store', '--export', *paths], stdout=subprocess.PIPE)
    unpacked = 0
    with archive.open('wb') as output:
        compressor = subprocess.Popen(['xz', '--threads=4', '-1', '--stdout'], stdin=subprocess.PIPE, stdout=output)
        try:
            while chunk := process.stdout.read(1024*1024):
                unpacked += len(chunk); compressor.stdin.write(chunk)
            compressor.stdin.close()
            if compressor.wait() or process.wait():
                raise RuntimeError('System export or compression failed')
        finally:
            for child in (process, compressor):
                if child.poll() is None:
                    child.kill(); child.wait()
    release = {'format': 1, 'version': version, 'arch': 'x86_64-linux', 'channel': 'preview', 'system': str(system),
               'sha256': digest(archive), 'bytes': archive.stat().st_size, 'unpackedBytes': unpacked,
               'url': FEED.rsplit('/', 1)[0]+'/'+version+'/system.nar.xz'}
    payload = folder / 'manifest.json'
    payload.write_text(json.dumps(release, sort_keys=True))
    signature = subprocess.check_output(['openssl', 'dgst', '-sha256', '-sign', str(key), str(payload)])
    envelope = {'payload': base64.b64encode(payload.read_bytes()).decode(), 'signature': base64.b64encode(signature).decode()}
    (destination/'latest.json').write_text(json.dumps(envelope)+'\n')
    print(json.dumps({'version': version, 'artifact': str(archive), 'bytes': release['bytes']}))


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('system', type=Path)
    parser.add_argument('destination', type=Path)
    parser.add_argument('--key', type=Path, required=True)
    args = parser.parse_args()
    build(args.system.resolve(), args.destination, args.key)
