"""Calamares adapter: use the same fail-closed offline installer as the CLI."""
import json
from pathlib import Path
import subprocess
import sys
import libcalamares

sys.path.insert(0, '@installerSource@')
from install import install_selected


def pretty_name():
    return 'Installing Anima.ing'


def run():
    gs = libcalamares.globalstorage
    try:
        if gs.value('partitionChoices') != {'install': 'erase', 'swap': 'none'}:
            raise ValueError('This preview supports dedicated-disk installation only.')
        if gs.value('username') != 'admin':
            raise ValueError('The local owner account must be admin.')
        devices = json.loads(Path('/run/anima-installer-disks.json').read_text())
        selected = [d for d in devices if d['path'] == gs.value('animaSelectedDisk')]
        if len(selected) != 1:
            raise ValueError('Disk was not present when setup started. Restart the installer.')
        password = libcalamares.utils.obscure(gs.value('password'))
        if len(password) < 12:
            raise ValueError('The owner password must contain at least 12 characters.')
        hashed = subprocess.run(['mkpasswd', '--method=yescrypt', '--stdin'],
                                input=password+'\n', text=True, capture_output=True, check=True).stdout.strip()
        del password
        gs.remove('password')
        keymap = subprocess.run(['ckbcomp', '-layout', gs.value('keyboardLayout') or 'us',
                                 '-variant', gs.value('keyboardVariant') or ''],
                                check=True, capture_output=True, text=True).stdout
        libcalamares.job.setprogress(0.05)
        install_selected('@installedSystem@', selected[0], hashed, keymap)
        libcalamares.job.setprogress(1.0)
        libcalamares.utils.debug('ANIMA_INSTALL_COMPLETE')
    except subprocess.CalledProcessError as error:
        # Report the operation and status, never its arguments or captured output.
        operation = Path(error.cmd[0]).name
        return ('Installation stopped', f'{operation} failed (exit {error.returncode}). Check the local system journal.')
    except (ValueError, OSError) as error:
        return ('Installation stopped', str(error))
    return None
