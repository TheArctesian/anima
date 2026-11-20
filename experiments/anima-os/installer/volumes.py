"""Explicit additional Btrfs volumes; never pool or discover disks to erase."""
import json
import subprocess
import time
from disks import inventory, validate


def validate_plan(system, data, devices):
    selected = [system, *data]
    if len({d['path'] for d in selected}) != len(selected):
        raise ValueError('A disk cannot have multiple installation roles.')
    # Validate every selected identity before the first destructive operation.
    return [validate(system, devices)] + [validate(d, devices, check_labels=False) for d in data]


def prepare_volumes(selected, persist):
    volumes = []
    for device in selected:
        disk = validate(device, inventory(), check_labels=False)
        subprocess.run(['wipefs', '--all', disk], check=True)
        subprocess.run(['parted', '--script', disk, 'mklabel', 'gpt', 'mkpart', 'AnimaData', 'btrfs', '1MiB', '100%'], check=True)
        subprocess.run(['udevadm', 'settle'], check=True)
        part = partition(disk)
        subprocess.run(['mkfs.btrfs', '-f', part], check=True)
        uuid = subprocess.check_output(['blkid', '-s', 'UUID', '-o', 'value', part], text=True).strip()
        volumes.append({'uuid': uuid, 'model': device.get('model'), 'serial': device.get('serial')})
    (persist / 'data-volumes.json').write_text(json.dumps(volumes)+'\n')


def partition(disk):
    for _ in range(20):
        for device in inventory():
            if device['path'] == disk:
                parts = [d['path'] for d in device.get('children', []) if d['type'] == 'part']
                if len(parts) == 1:
                    return parts[0]
        time.sleep(0.25)
    raise ValueError('The selected data partition did not appear.')
