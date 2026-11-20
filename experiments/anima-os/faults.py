"""Independent crash/pressure experiments; failures are recorded, never hidden."""
import argparse
import json
from pathlib import Path
import time
import uuid
from vm import Machine, digest, prepare


def case(image, directory, scenario):
    disk, sentinel = prepare(image, directory)
    machine = Machine(directory, disk)
    outcome = {'case': scenario, 'result': 'Failed', 'harness_reset': False}
    try:
        machine.ready()
        nonce = uuid.uuid4().hex
        machine.command(f'printf %s {nonce} > /persist/workload/sentinel; sha256sum /persist/workload/sentinel > /var/lib/anima-prototype/crash.sha256')
        if scenario == 'staging-cut':
            machine.command('dd if=/boot/EFI/anima/b-initrd of=/persist/staged-initrd.part bs=1M count=1 status=none; sync')
        elif scenario == 'publication-cut':
            machine.command("mv /boot/loader/entries/anima-b.conf /boot/loader/entries/anima-b+2.conf; printf 'default anima-*.conf\\ntimeout 1\\n' > /boot/loader/loader.conf; sync")
        elif scenario == 'pre-health-cut':
            machine.command("sed -i '/^options /s/$/ systemd.mask=anima-local-health.service/' /boot/loader/entries/anima-b.conf; mv /boot/loader/entries/anima-b.conf /boot/loader/entries/anima-b+2.conf; printf 'default anima-*.conf\\ntimeout 1\\n' > /boot/loader/loader.conf; sync")
            machine.send('systemctl reboot')
            machine.expect('<<< Welcome to NixOS')
            machine.expect(r'root@anima-prototype:[^\r\n]*#')
            machine.send('stty -echo')
            machine.command("test -f /boot/loader/entries/anima-b+1-1.conf; sed -i 's/ systemd.mask=anima-local-health.service//' /boot/loader/entries/anima-b+1-1.conf; sync")
        elif scenario == 'both-health-fail':
            machine.command("touch /persist/fail-all-health; mv /boot/loader/entries/anima-b.conf /boot/loader/entries/anima-b+2.conf; printf 'default anima-*.conf\\ntimeout 1\\n' > /boot/loader/loader.conf; sync")
            machine.send('systemctl reboot')
            for _ in range(3):
                machine.expect('ANIMA_HEALTH_FAILED')
            machine.expect('ANIMA_RECOVERY_REQUIRED')
            machine.expect(r'root@anima-prototype:[^\r\n]*#')
            machine.send('stty -echo')
            machine.command('cat /proc/sys/kernel/random/boot_id > /run/recovery-boot; sleep 25; cmp /proc/sys/kernel/random/boot_id /run/recovery-boot')
        elif scenario == 'rejected-entry':
            machine.command("sed -i 's@linux .*@linux /EFI/anima/missing-kernel@' /boot/loader/entries/anima-b.conf; mv /boot/loader/entries/anima-b.conf /boot/loader/entries/anima-b+2.conf; printf 'default anima-*.conf\\ntimeout 1\\n' > /boot/loader/loader.conf; sync")
            machine.reboot('A')
        elif scenario == 'reboot-deadline-idle':
            machine.command('cat /proc/sys/kernel/random/boot_id > /run/idle-boot; sleep 65; cmp /proc/sys/kernel/random/boot_id /run/idle-boot', timeout=75)
        elif scenario == 'shutdown-hang':
            machine.command("systemd-run --unit=anima-stuck-stop --property=TimeoutStopSec=infinity --property=KillSignal=SIGCONT --property=SendSIGKILL=no sleep infinity")
            started = time.monotonic()
            machine.send('systemctl reboot')
            machine.ready('A')
            outcome['recovery_seconds'] = time.monotonic() - started
        elif scenario in ('initrd-hang', 'initrd-emergency-policy'):
            if scenario == 'initrd-emergency-policy':
                machine.command("sed -i '/^options /s/$/ boot.panic_on_fail/' /boot/loader/entries/anima-b.conf")
            machine.command("sed -i '/^options /s/$/ rd.systemd.unit=emergency.target/' /boot/loader/entries/anima-b.conf; mv /boot/loader/entries/anima-b.conf /boot/loader/entries/anima-b+2.conf; printf 'default anima-*.conf\\ntimeout 1\\n' > /boot/loader/loader.conf; sync")
            machine.reboot('A')
        elif scenario == 'capacity':
            machine.command('if dd if=/dev/zero of=/persist/workload/disposable bs=64M status=none 2>/run/fill.error; then exit 99; fi; grep -q "No space left on device" /run/fill.error', timeout=120)
            started = time.monotonic()
            machine.command('sha256sum -c /var/lib/anima-prototype/crash.sha256; findmnt -n -o OPTIONS /; btrfs filesystem usage /persist; rm /persist/workload/disposable; btrfs filesystem sync /persist')
            outcome.update(response_seconds=time.monotonic()-started,
                           limitation='Read/cleanup smoke only; no allocation admission control implemented')
        else:
            raise ValueError(scenario)
        if scenario.endswith('-cut'):
            machine.process.kill()  # Deliberate guest power-loss injection, not recovery evidence.
            machine.process.wait()
            machine.close()
            outcome['harness_reset'] = True
            machine = Machine(directory, disk)
            machine.ready('A' if scenario == 'staging-cut' else 'B')
        machine.command('sha256sum -c /var/lib/anima-prototype/crash.sha256')
        outcome['result'] = 'Passed'
    except Exception as error:
        outcome['reason'] = str(error)
    finally:
        machine.close()
        outcome['unselected_disk_unchanged'] = digest(directory / 'sentinel.raw') == sentinel
        if not outcome['unselected_disk_unchanged']:
            outcome['result'] = 'Failed'
        (directory / 'result.json').write_text(json.dumps(outcome, indent=2) + '\n')
    return outcome


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('image', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--scenario', choices=['staging-cut', 'publication-cut', 'rejected-entry', 'initrd-hang', 'capacity', 'pre-health-cut', 'both-health-fail', 'shutdown-hang', 'initrd-emergency-policy', 'reboot-deadline-idle'], required=True)
    parser.add_argument('--iterations', type=int, default=3)
    args = parser.parse_args()
    if args.iterations < 1:
        parser.error('--iterations must be positive')
    args.output.mkdir(parents=True, exist_ok=False)
    results = []
    for iteration in range(args.iterations):
        results.append(case(args.image.resolve(), args.output / str(iteration+1), args.scenario))
        (args.output / 'results.json').write_text(json.dumps(results, indent=2) + '\n')
        if results[-1]['result'] != 'Passed':
            break  # Diagnose before repeating an identical failure.

    if any(result['result'] != 'Passed' for result in results):
        raise SystemExit(1)
