"""Publish versioned EFI assets before selecting a candidate; keep the current system pinned."""
import os
from pathlib import Path
import shutil
import subprocess

DIRECTORY = Path('/persist/anima-updates')
ENTRIES = Path('/boot/loader/entries')


def atomic(path, value):
    temporary = path.with_name(path.name+'.tmp')
    with temporary.open('w') as output:
        output.write(value); output.flush(); os.fsync(output.fileno())
    temporary.replace(path)
    descriptor = os.open(path.parent, os.O_DIRECTORY)
    try:
        os.fsync(descriptor)
    finally:
        os.close(descriptor)


def pin(name, system):
    root = Path('/nix/var/nix/gcroots/anima-'+name)
    temporary = root.with_name(root.name+'.tmp')
    temporary.unlink(missing_ok=True)
    temporary.symlink_to(system)
    temporary.replace(root)


def entry(system, name, title, version):
    source = Path(system)
    destination = Path('/boot/EFI/anima') / source.name.split('-')[0]
    destination.mkdir(parents=True, exist_ok=True)
    for asset in ('kernel', 'initrd'):
        temporary = destination / (asset+'.tmp')
        shutil.copyfile(source / asset, temporary)
        temporary.replace(destination / asset)
    os.sync()
    prefix = '/EFI/anima/'+destination.name
    params = (source/'kernel-params').read_text().strip()
    atomic(ENTRIES / name, f'title {title}\nsort-key zz-anima\nversion {version}\nlinux {prefix}/kernel\ninitrd {prefix}/initrd\noptions init={system}/init {params}\n')


def select(pattern):
    atomic(Path('/boot/loader/loader.conf'), f'default {pattern}\ntimeout 5\neditor no\n')
    # Clear a previous firmware override so loader.conf selects the intended entry.
    subprocess.run(['bootctl', 'set-default', ''], check=True, capture_output=True)
    os.sync()


def stage(system):
    current = str(Path('/run/current-system').resolve())
    pin('previous', current); pin('candidate', system)
    entry(current, 'anima-current.conf', 'Anima OS · previous system', 1)
    select('anima-current.conf')
    for old in ENTRIES.glob('anima-candidate*.conf'):
        old.unlink()
    entry(system, 'anima-candidate+2.conf', 'Anima OS · preview update', 2)
    return current


def activate():
    if not (ENTRIES/'anima-candidate+2.conf').exists():
        raise ValueError('No staged update is ready to restart.')
    select('anima-*.conf')
    subprocess.run(['systemctl', '--no-block', 'reboot'], check=True)
