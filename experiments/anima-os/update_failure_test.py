"""Break a real candidate's local service in a disposable VM and require bounded fallback."""
import argparse
import json
from pathlib import Path
from vm import Machine
from update_test import login
import update_fixture


def run(output):
    password=(output/'fixture-password').read_text()
    machine=Machine(output,output/'installed.qcow2')
    result={'case':'failed-local-service-bounded-fallback','result':'Failed'}
    try:
        login(machine,password)
        machine.command('cmp /persist/update-before <(readlink -f /run/current-system)')
        update_fixture.attach(machine,output,output/'update-origin.raw')
        machine.command('SSL_CERT_FILE=/run/update-origin/certificate.pem anima-os-update download',timeout=1800)
        machine.command("sed -i '/^options /s/$/ systemd.mask=NetworkManager.service/' /boot/loader/entries/anima-candidate+2.conf; sync")
        machine.send('anima-os-update activate')
        for _ in range(3):
            machine.expect('reboot: Restarting system',timeout=180)
        login(machine,password)
        machine.command('cmp /persist/update-before <(readlink -f /run/current-system); cmp /persist/update-identity /etc/machine-id; grep -qx keep-work /persist/update-work; systemctl is-active --quiet NetworkManager; test -f /boot/loader/entries/anima-candidate+0-2.conf')
        machine.command("timeout 30 sh -c 'until anima-os-update status | jq -e \".value.phase == \\\"rolled-back\\\"\"; do sleep 1; done'",timeout=35)
        result['result']='Passed'
    except Exception as error:
        result['reason']=str(error)
    finally:
        machine.close()
        (output/'failure-recovery.json').write_text(json.dumps(result,indent=2)+'\n')
    if result['result']!='Passed':
        raise SystemExit(1)


if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('output',type=Path)
    run(parser.parse_args().output.resolve())
