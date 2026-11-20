"""Explicitly confirmed, single-disk offline installation for the preview ISO."""
import getpass
import os
from pathlib import Path
import subprocess
import sys
import time
from disks import eligible, inventory, validate

ROOT = Path('/mnt/anima-install')


def run(*args, **kwargs):
    return subprocess.run(args, check=True, **kwargs)


def choose():
    devices = inventory()
    choices = [device for device in devices if eligible(device)]
    if not choices:
        raise ValueError('No unused writable whole disk of at least 32 GiB found.')
    print('\nAnima OS experimental UEFI installer\n')
    print('The selected disk will be erased. Other disks will not be formatted.')
    print('Preview: no disk encryption, BIOS/ARM, managed jobs/agents or automatic backup.')
    print('Installation is local; hub-driven setup is not implemented yet.\n')
    for number, disk in enumerate(choices, 1):
        print(f"{number}. {disk['path']}  {int(disk['size']) // 1024**3} GiB  {disk.get('model') or ''}  serial: {disk.get('serial') or 'unknown'}")
    selection = input('\nDisk number (empty cancels): ').strip()
    if not selection.isdigit() or not 1 <= int(selection) <= len(choices):
        raise ValueError('Cancelled; no disk changed.')
    return choices[int(selection)-1]


def password_hash():
    first = getpass.getpass('Password for the local admin account (at least 12 characters): ')
    second = getpass.getpass('Repeat password: ')
    if len(first) < 12 or first != second:
        raise ValueError('Passwords differ or are too short; no disk changed.')
    result = run('mkpasswd', '--method=yescrypt', '--stdin', input=first+'\n', text=True, capture_output=True)
    return result.stdout.strip()


def write_password(path, hashed):
    with open(path, 'x', opener=lambda name, flags: os.open(name, flags, 0o600)) as output:
        output.write(hashed+'\n')


def partitions(disk):
    # Discover by parent after the kernel has accepted the new GPT.
    for _ in range(20):
        for device in inventory():
            if device['path'] == disk:
                parts = [child['path'] for child in device.get('children', []) if child['type'] == 'part']
                if len(parts) == 2:
                    return parts
        time.sleep(0.25)
    raise ValueError('New partitions did not appear; installation stopped.')


def install(system):
    if os.geteuid() != 0 or not Path('/sys/firmware/efi').exists():
        raise ValueError('Run from the live ISO as root after booting in UEFI mode.')
    if not Path(system, 'bin/switch-to-configuration').is_file():
        raise ValueError('The offline system closure is missing.')
    if ROOT.exists():
        raise ValueError('Installation mount directory exists; reboot the live ISO before trying again.')
    selected = choose()
    hashed = password_hash()
    phrase = f"ERASE {selected['path']}"
    if input(f'\nTo erase this disk, type exactly {phrase}: ') != phrase:
        raise ValueError('Cancelled; no disk changed.')
    disk = validate(selected, inventory())
    # No mutation takes place before this point. Revalidate after all prompts.
    ROOT.mkdir(parents=True)
    mounted = False
    try:
        run('wipefs', '--all', disk)
        run('parted', '--script', disk, 'mklabel', 'gpt', 'mkpart', 'ESP', 'fat32', '1MiB', '1025MiB', 'set', '1', 'esp', 'on', 'mkpart', 'Anima', 'btrfs', '1025MiB', '100%')
        run('udevadm', 'settle')
        esp, data = partitions(disk)
        run('mkfs.vfat', '-n', 'ANIMA_EFI', esp)
        run('mkfs.btrfs', '-f', '-L', 'ANIMA_OS', data)
        run('mount', '-t', 'btrfs', data, str(ROOT)); mounted = True
        for name in ('root', 'nix', 'var', 'persist'):
            run('btrfs', 'subvolume', 'create', str(ROOT / f'@{name}'))
        run('umount', str(ROOT)); mounted = False
        run('mount', '-t', 'btrfs', '-o', 'subvol=@root,compress=zstd', data, str(ROOT)); mounted = True
        for name in ('nix', 'var', 'persist', 'boot'):
            (ROOT / name).mkdir()
        for name in ('nix', 'var', 'persist'):
            run('mount', '-t', 'btrfs', '-o', f'subvol=@{name},compress=zstd', data, str(ROOT / name))
        run('mount', '-t', 'vfat', '-o', 'umask=0077', esp, str(ROOT / 'boot'))
        (ROOT / 'persist/network-connections').mkdir(mode=0o700)
        (ROOT / 'persist/home').mkdir(mode=0o755)
        secret = ROOT / 'persist/owner-password'
        write_password(secret, hashed)
        run('nixos-install', '--root', str(ROOT), '--system', system, '--no-root-passwd', '--no-channel-copy', '--option', 'substituters', '')
        run('sync')
    finally:
        if mounted:
            run('umount', '--recursive', str(ROOT))
        ROOT.rmdir()
    print('\nANIMA_INSTALL_COMPLETE\nRemove the USB and reboot. Log in as admin with your chosen password.')


if __name__ == '__main__':
    try:
        if len(sys.argv) != 2:
            raise ValueError('Start with anima-install from the live ISO.')
        install(sys.argv[1])
    except (ValueError, OSError, subprocess.CalledProcessError, KeyboardInterrupt, EOFError) as error:
        print(f'Installation stopped: {error}', file=sys.stderr)
        sys.exit(1)
