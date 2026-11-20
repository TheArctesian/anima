"""Explicit local-owner enrollment against an existing private HTTPS hub."""
from pathlib import Path
import urllib.parse
from commands import run


def enroll(args):
    if Path('/var/lib/anima-daemon/daemon.json').exists():
        raise ValueError('This computer is already enrolled. Existing ownership will not be replaced.')
    hub, code = args.get('hub', '').strip(), args.get('code', '').strip()
    parsed = urllib.parse.urlsplit(hub)
    if (parsed.scheme != 'https' or not parsed.hostname or parsed.port not in (None, 443)
            or parsed.username or parsed.password or parsed.path not in ('', '/') or parsed.query or parsed.fragment
            or '\\' in hub or any(char.isspace() for char in hub)):
        raise ValueError('Use the HTTPS origin of your hub on port 443, without a path or credentials.')
    if not code.startswith('anima_enrol_') or len(code) > 128 or any(char.isspace() for char in code):
        raise ValueError('Enter the enrollment code from this device’s page in your hub.')
    run(['runuser', '-u', 'anima-daemon', '--', 'env', 'ANIMA_DAEMON_HOME=/var/lib/anima-daemon',
         'anima-daemon', 'enrol', '-', '--hub', hub], code+'\n', 45)
    run(['systemctl', 'restart', 'anima-daemon'])
    return {'enrolled': True}
