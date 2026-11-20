"""Install the actual ISO, update from signed HTTPS bytes, reboot and roll back without data loss."""
import argparse
import json
import os
from pathlib import Path
import secrets
from gui_test import prepare, graphical_install, installed_checks, screenshot, eject_install_media
from gui_actions import tap
from vm import Machine, digest
import update_fixture


def root(machine,password):
    machine.send('sudo -i');machine.expect('password for admin:');machine.send(password)
    machine.expect(r'root@anima(?:-node-[a-z0-9]{4})?:[^\r\n]*#');machine.send('stty -echo')


def login(machine,password):
    machine.expect('anima(?:-node-[a-z0-9]{4})? login:',timeout=240);machine.send('admin')
    machine.expect('Password:');machine.send(password)
    machine.expect(r'admin@anima(?:-node-[a-z0-9]{4})?:[^\r\n]*\$');machine.send('stty -echo')
    root(machine,password)


def run(iso,package,output,resume=False):
    if resume:
        disk=output/'installed.qcow2';fixture=output/'update-origin.raw'
        password=(output/'fixture-password').read_text()
    else:
        disk=prepare(output);fixture=update_fixture.prepare(package,output)
        password=secrets.token_hex(12)
        with open(os.open(output/'fixture-password',os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o600),'w') as secret:
            secret.write(password)
    before=digest(output/'sentinel.raw')
    results=[];machine=Machine(output,disk,boot_iso=None if resume else iso)
    try:
        if resume:
            login(machine,password)
        else:
            graphical_install(machine,output,password)
            tap(machine,output,'Restart')
            machine.expect('reboot: Restarting system', timeout=90)
            eject_install_media(output)
            installed_checks(machine,password,output)
            results.append({'case':'completion-restart-installed-console-storage-wire','result':'Passed'})
            root(machine,password)
        machine.command('readlink -f /run/current-system > /persist/update-before; cp /etc/machine-id /persist/update-identity; cp /etc/hostname /persist/update-hostname; echo keep-work > /persist/update-work')
        update_fixture.attach(machine,output,fixture)
        machine.command('SSL_CERT_FILE=/run/update-origin/certificate.pem anima-os-update check > /tmp/check.json || { cat /tmp/check.json; journalctl -u anima-update-fixture --no-pager -n 12; exit 1; }; jq -e ".ok and .value.available" /tmp/check.json',timeout=45)
        machine.command('SSL_CERT_FILE=/run/update-origin/certificate.pem anima-os-update download > /tmp/stage.json; jq -e ".ok and .value.phase == \\"staged\\"" /tmp/stage.json',timeout=1800)
        machine.command('cmp /persist/update-before <(readlink -f /run/current-system); grep -qx keep-work /persist/update-work')
        results.append({'case':'signed-https-download-staging-without-live-activation','result':'Passed'})
        machine.send('anima-os-update activate')
        login(machine,password)
        machine.command("timeout 90 sh -c 'until anima-os-update status | jq -e \".value.phase == \\\"current\\\"\"; do sleep 2; done'",timeout=95)
        machine.command('! cmp -s /persist/update-before <(readlink -f /run/current-system); cmp /persist/update-identity /etc/machine-id; cmp /persist/update-hostname /proc/sys/kernel/hostname; grep -qx keep-work /persist/update-work; findmnt -n -o OPTIONS / | tr , "\\n" | grep -qx ro')
        results.append({'case':'updated-system-health-and-persistence','result':'Passed'})
        machine.send('anima-os-update rollback')
        login(machine,password)
        machine.command('cmp /persist/update-before <(readlink -f /run/current-system); cmp /persist/update-identity /etc/machine-id; cmp /persist/update-hostname /proc/sys/kernel/hostname; grep -qx keep-work /persist/update-work')
        results.append({'case':'owner-rollback-preserves-data','result':'Passed'})
    except Exception as error:
        results.append({'case':'update-run','result':'Failed','reason':str(error)})
        try:
            machine.command('cat /tmp/check.json /tmp/stage.json 2>/dev/null || true; journalctl -b -u anima-update-health -u anima-update-fixture --no-pager -n 20')
        except Exception:
            pass
        screenshot(output,'failure')
    finally:
        machine.close()
        results.append({'case':'unselected-disk','result':'Passed' if digest(output/'sentinel.raw')==before else 'Failed'})
        (output/'results.json').write_text(json.dumps({'iso_sha256':digest(iso),'results':results},indent=2)+'\n')
    if any(entry['result']!='Passed' for entry in results):
        raise SystemExit(1)


if __name__=='__main__':
    parser=argparse.ArgumentParser()
    parser.add_argument('iso',type=Path);parser.add_argument('package',type=Path);parser.add_argument('output',type=Path)
    parser.add_argument('--resume',action='store_true')
    args=parser.parse_args();run(args.iso.resolve(),args.package.resolve(),args.output.resolve(),args.resume)
