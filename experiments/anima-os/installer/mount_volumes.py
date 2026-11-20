"""Mount only UUIDs recorded by the installer; missing disks do not block boot."""
import json
from pathlib import Path
import subprocess
import uuid

records = Path('/persist/data-volumes.json')
if records.exists():
    for volume in json.loads(records.read_text()):
        identifier = str(uuid.UUID(volume['uuid']))
        source = Path('/dev/disk/by-uuid') / identifier
        target = Path('/persist/volumes') / identifier
        if not source.exists():
            print(f'Data volume {identifier} is missing; leaving it unmounted.', flush=True)
            continue
        target.mkdir(parents=True, exist_ok=True)
        if not target.is_mount():
            subprocess.run(['mount', '-t', 'btrfs', '-o', 'compress=zstd,nosuid,nodev', str(source), str(target)], check=True)
