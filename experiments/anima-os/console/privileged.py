"""A private stdin/stdout session, started only after sudo authenticates the local owner."""
import fcntl
import json
import os
from pathlib import Path
import subprocess
import sys
import threading
from commands import connect, networks, run, status
from hosted import Account
from orientation import outputs, rotate


class Session:
    def __init__(self):
        self.account = Account()
        self.rotation = None
        self.revert_timer = None
        self.rotation_lock = threading.RLock()

    def revert(self):
        with self.rotation_lock:
            if self.revert_timer:
                self.revert_timer.cancel()
            if self.rotation:
                output, previous = self.rotation
                self.rotation = None
                rotate(output, previous)

    def display(self, args):
        with self.rotation_lock:
            self.revert()
            active = outputs(run(['xrandr', '--prop']))
            selected = next((item for item in active if item['name'] == args['output']), None)
            if not selected or args['rotation'] not in ('normal', 'left', 'right', 'inverted'):
                raise ValueError('Choose an available display and orientation.')
            self.rotation = (selected['name'], selected['rotation'])
            self.revert_timer = threading.Timer(15, self.revert)
            self.revert_timer.daemon = True
            self.revert_timer.start()
            rotate(selected['name'], args['rotation'])
            return {'revertSeconds': 15}

    def dispatch(self, op, args):
        if op == 'status':
            return status()
        if op == 'scan':
            return networks()
        if op == 'connect':
            return connect(args)
        if op == 'displays':
            return outputs(run(['xrandr', '--prop']))
        if op == 'rotate':
            return self.display(args)
        if op == 'keep-rotation':
            with self.rotation_lock:
                if self.rotation is None:
                    raise ValueError('The display preview expired. Apply it again.')
                self.revert_timer.cancel()
                self.rotation = None
            return {}
        if op == 'revert':
            self.revert()
            return {}
        if op == 'sign-in':
            return self.account.start()
        if op == 'sign-in-poll':
            return self.account.poll()
        if op == 'cancel-sign-in':
            self.account = Account()
            return {}
        if op == 'enroll':
            with open('/run/anima-console-enrollment.lock', 'w') as lease:
                try:
                    fcntl.flock(lease, fcntl.LOCK_EX | fcntl.LOCK_NB)
                except BlockingIOError:
                    raise ValueError('Enrollment is already running. Wait for it to finish.') from None
                return self.account.enroll()
        if op in ('restart', 'shutdown'):
            if args.get('confirmed') is not True:
                raise ValueError('Confirm the power action first.')
            run(['systemctl', '--no-block', 'reboot' if op == 'restart' else 'poweroff'])
            return {'requested': True}
        if op == 'diagnostics':
            return {'network': run(['nmcli', '-t', '-f', 'STATE,CONNECTIVITY', 'general']),
                    'radio': run(['rfkill', '--json']), 'root': run(['findmnt', '-n', '-o', 'OPTIONS', '/']),
                    'monitoring': status()['daemon']}
        raise ValueError('Unsupported operation.')


def main():
    if os.geteuid() != 0:
        raise SystemExit('Local owner authentication required.')
    os.umask(0o077)
    session = Session()
    print(json.dumps({'id': 'unlock', 'ok': True, 'value': {}}), flush=True)
    try:
        while line := sys.stdin.buffer.readline(65537):
            if len(line) > 65536:
                break
            request = json.loads(line)
            try:
                value = session.dispatch(request['op'], request.get('args', {}))
                answer = {'id': request['id'], 'ok': True, 'value': value}
            except ValueError as error:
                answer = {'id': request['id'], 'ok': False, 'error': str(error)}
            except Exception:
                answer = {'id': request['id'], 'ok': False, 'error': 'The operation could not finish. Check the connection and retry.'}
            print(json.dumps(answer), flush=True)
    finally:
        session.revert()


if __name__ == '__main__':
    main()
