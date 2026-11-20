"""Authenticate the hosted preview manifest before trusting any release fields."""
import base64
import hashlib
import json
from pathlib import Path
import re
import subprocess
import tempfile
import urllib.request

FEED = 'https://releases.anima.ing/os/preview/x86_64-linux/latest.json'
KEY = Path(__file__).with_name('preview-public.pem')


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None


def fetch(url, output, limit):
    if not url.startswith('https://releases.anima.ing/os/preview/x86_64-linux/'):
        raise ValueError('The update address is outside the Anima OS preview channel.')
    with urllib.request.build_opener(NoRedirect()).open(url, timeout=30) as response:
        count = 0
        while chunk := response.read(1024 * 1024):
            count += len(chunk)
            if count > limit:
                raise ValueError('The update exceeded its declared size.')
            output.write(chunk)
    return count


def verify(envelope, key=KEY):
    try:
        signed = json.loads(envelope)
        payload = base64.b64decode(signed['payload'], validate=True)
        signature = base64.b64decode(signed['signature'], validate=True)
        with tempfile.TemporaryDirectory() as directory:
            data, proof = Path(directory)/'payload', Path(directory)/'signature'
            data.write_bytes(payload); proof.write_bytes(signature)
            result = subprocess.run(['openssl', 'dgst', '-sha256', '-verify', str(key), '-signature', str(proof), str(data)], capture_output=True)
            if result.returncode:
                raise ValueError('Update signature verification failed.')
        release = json.loads(payload)
        if (release['format'] != 1 or release['arch'] != 'x86_64-linux' or release['channel'] != 'preview'
                or not re.fullmatch(r'[0-9]{14}', release['version'])
                or not re.fullmatch(r'/nix/store/[a-z0-9]{32}-nixos-system-anima-[A-Za-z0-9.+_-]+', release['system'])
                or not re.fullmatch(r'[a-f0-9]{64}', release['sha256'])
                or not isinstance(release['bytes'], int) or not 0 < release['bytes'] <= 32 * 1024**3
                or not isinstance(release['unpackedBytes'], int) or not 0 < release['unpackedBytes'] <= 80 * 1024**3):
            raise ValueError('The signed release is not compatible with this preview.')
        expected = FEED.rsplit('/', 1)[0]+'/'+release['version']+'/system.nar.xz'
        if release['url'] != expected:
            raise ValueError('The release artifact address is invalid.')
        return release
    except (KeyError, TypeError, json.JSONDecodeError) as error:
        raise ValueError('The update feed is invalid.') from error


def digest(path):
    with path.open('rb') as source:
        return hashlib.file_digest(source, 'sha256').hexdigest()
