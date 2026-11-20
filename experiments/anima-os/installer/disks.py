"""Read-only disk inventory and fail-closed installation selection."""
import json
import os
from pathlib import Path
import stat
import subprocess


def inventory():
    result = subprocess.run(['lsblk', '--json', '--tree', '--bytes', '--paths', '--output',
                             'PATH,TYPE,SIZE,MODEL,SERIAL,RO,MOUNTPOINTS,LABEL,MAJ:MIN'],
                            check=True, capture_output=True, text=True)
    devices = json.loads(result.stdout)['blockdevices']
    if any(device['type'] == 'part' for device in devices):
        raise ValueError('Disk inventory is not a complete dependency tree; refusing installation.')
    return devices


def descendants(device):
    yield device
    for child in device.get('children', []):
        yield from descendants(child)


def busy(device):
    return any(any(node.get('mountpoints') or []) or node.get('type') not in ('disk', 'part')
               for node in descendants(device))


def eligible(device):
    return (device['type'] == 'disk' and not device['ro'] and
            int(device['size']) >= 32 * 1024**3 and not busy(device))


def identity(device):
    return tuple(device.get(field) for field in ('path', 'maj:min', 'size', 'model', 'serial'))


def validate(selected, devices):
    candidates = [device for device in devices if identity(device) == identity(selected)]
    if len(candidates) != 1 or not eligible(candidates[0]):
        raise ValueError('Selected disk changed or became busy; start selection again.')
    # The preview uses fixed labels. Never install beside another matching OS.
    for device in devices:
        if device['path'] != selected['path']:
            if any(node.get('label') in ('ANIMA_OS', 'ANIMA_EFI') for node in descendants(device)):
                raise ValueError('Another disk already has Anima labels; disconnect it before installing.')
    path = Path(selected['path'])
    if path.is_symlink() or not stat.S_ISBLK(path.stat().st_mode):
        raise ValueError('Target must be a real whole block device, not a link or file.')
    if os.major(path.stat().st_rdev) != int(selected['maj:min'].split(':')[0]) or os.minor(path.stat().st_rdev) != int(selected['maj:min'].split(':')[1]):
        raise ValueError('Block device identity changed.')
    return selected['path']
