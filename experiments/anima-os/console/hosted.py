"""Account sign-in and automatic enrollment using the existing cloud, link and hub contracts."""
import http.client
import ipaddress
import json
import os
from pathlib import Path
import socket
import time
import urllib.error
import urllib.parse
import urllib.request
from commands import run

CLIENT = 'client_01M7M1VFKREYV6H5714SDVS4E6'
CLOUD = 'https://cloud.anima.ing'
STATE = '/var/lib/anima-link'
SOCKET = '/var/lib/anima-daemon/link.sock'


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None


def request(url, body, token=None, form=False):
    data = urllib.parse.urlencode(body).encode() if form else json.dumps(body).encode()
    headers = {'Content-Type': 'application/x-www-form-urlencoded' if form else 'application/json'}
    if token:
        headers['Authorization'] = 'Bearer '+token
    req = urllib.request.Request(url, data=data, headers=headers)
    try:
        with urllib.request.build_opener(NoRedirect()).open(req, timeout=20) as response:
            return json.loads(response.read(1048576))
    except urllib.error.HTTPError as error:
        if error.code < 500:
            answer = json.loads(error.read(65536))
            return {'failure': answer.get('error', 'refused')}
        raise ValueError('The account service is unavailable. Try again later.') from None


def graph(query, variables=None, token=None):
    connection = http.client.HTTPConnection('localhost', timeout=25)
    connection.sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    connection.sock.settimeout(25)
    connection.sock.connect(SOCKET)
    headers = {'Content-Type': 'application/json'}
    if token:
        headers['Authorization'] = 'Bearer '+token
    try:
        connection.request('POST', '/graphql', json.dumps({'query': query, 'variables': variables or {}}), headers)
        reply = connection.getresponse()
        body = json.loads(reply.read(1048576))
        if reply.status != 200 or body.get('errors'):
            raise ValueError('The hub refused this step. Your existing enrollment was not replaced.')
        return body['data']
    finally:
        connection.close()


class Account:
    def __init__(self):
        self.pending = None
        self.access = None
        self.email = ''
        self.next_poll = 0
        self.deadline = 0
        self.interval = 5

    def start(self):
        self.access = None
        answer = request('https://api.workos.com/user_management/authorize/device', {'client_id': CLIENT}, form=True)
        if 'failure' in answer:
            raise ValueError('Sign-in is unavailable for this account configuration.')
        url = answer['verification_uri_complete']
        parsed = urllib.parse.urlsplit(url)
        if parsed.scheme != 'https' or not parsed.hostname or parsed.username or parsed.password:
            raise ValueError('The sign-in service returned an invalid browser address.')
        self.pending = answer['device_code']
        self.interval = max(5, int(answer.get('interval', 5)))
        self.next_poll = time.monotonic()+self.interval
        self.deadline = time.monotonic()+min(900, int(answer.get('expires_in', 600)))
        return {'url': url, 'userCode': answer['user_code'], 'waiting': True}

    def poll(self):
        if not self.pending or time.monotonic() > self.deadline:
            self.pending = None
            raise ValueError('Sign-in expired. Start again.')
        if time.monotonic() < self.next_poll:
            return {'waiting': True}
        answer = request('https://api.workos.com/user_management/authenticate',
                         {'client_id': CLIENT, 'grant_type': 'urn:ietf:params:oauth:grant-type:device_code',
                          'device_code': self.pending}, form=True)
        failure = answer.get('failure')
        if failure in ('authorization_pending', 'slow_down'):
            if failure == 'slow_down':
                self.interval += 5
            self.next_poll = time.monotonic()+self.interval
            return {'waiting': True}
        self.pending = None
        if failure:
            raise ValueError('Sign-in was not completed. Start again.')
        self.access = answer['access_token']
        self.email = answer.get('user', {}).get('email', 'Signed-in Anima account')
        return {'signedIn': True, 'email': self.email}

    def enroll(self):
        if not self.access:
            raise ValueError('Sign in before adding this computer.')
        if Path('/var/lib/anima-daemon/daemon.json').exists():
            raise ValueError('This computer is already enrolled. Existing ownership will not be replaced.')
        machine = Path('/etc/machine-id').read_text().strip()
        name = 'anima-'+machine[:12]
        deadline = time.monotonic()+90
        while True:
            ready = request(CLOUD+'/v1/hub', {}, self.access)
            if ready.get('state') == 'ready':
                break
            if ready.get('state') not in ('making', 'waking') or time.monotonic() >= deadline:
                raise ValueError('Your hub is not ready yet. Wait a moment and sign in again.')
            time.sleep(2)
        access = request(CLOUD+'/v1/hub/access', {'device': name}, self.access)
        if 'failure' in access:
            raise ValueError('Your account could not authorize this computer.')
        invite = access.get('invite')
        if invite:
            run(['runuser', '-u', 'anima-daemon', '--', 'anima-link', 'join', '-state', STATE], invite+'\n', 45)
        elif not Path(STATE+'/device.json').exists():
            raise ValueError('The account expects an existing link identity, but this disk does not have it.')
        run(['systemctl', 'restart', 'anima-link'], timeout=45)
        deadline = time.monotonic()+30
        while not Path(SOCKET).exists() and time.monotonic() < deadline:
            time.sleep(.2)
        token = graph('mutation($claim:String!){claimDeviceToken(claim:$claim){token}}', {'claim': access['claim']})['claimDeviceToken']['token']
        try:
            device_id = self.device(token, name)
            code = graph('mutation($id:ID!){enrolDaemon(deviceId:$id){code}}', {'id': device_id}, token)['enrolDaemon']['code']
            run(['runuser', '-u', 'anima-daemon', '--', 'env', 'ANIMA_DAEMON_HOME=/var/lib/anima-daemon',
                 'anima-daemon', 'enrol', '-', '--hub', 'unix:'+SOCKET], code+'\n', 45)
            run(['systemctl', 'restart', 'anima-daemon'])
            return {'enrolled': True}
        finally:
            try:
                graph('mutation{logout}', token=token)
            except (ValueError, OSError, http.client.HTTPException):
                pass
            self.access = None

    def device(self, token, name):
        linked = graph('query{linkDevices{name address}}', token=token)['linkDevices']
        matches = [entry for entry in linked if entry['name'] == name]
        if len(matches) != 1:
            raise ValueError('The hub could not uniquely identify this computer.')
        address = str(ipaddress.ip_address(matches[0]['address']))
        known = graph('query{sites{id name subnet}}', token=token)
        devices = []
        cursor = None
        while True:
            batch = graph('query($after:String){devices(first:100,after:$after){edges{node{id host}} pageInfo{hasNextPage endCursor}}}', {'after': cursor}, token)['devices']
            devices.extend(edge['node'] for edge in batch['edges'])
            if not batch['pageInfo']['hasNextPage']:
                break
            next_cursor = batch['pageInfo']['endCursor']
            if not next_cursor or next_cursor == cursor:
                raise ValueError('The device inventory could not be read completely.')
            cursor = next_cursor
        for device in devices:
            try:
                if str(ipaddress.ip_address(device['host'])) == address:
                    return device['id']
            except ValueError:
                continue
        sites = [site for site in known['sites'] if site['subnet'] == 'fd41:4e49:4d41::/48']
        if sites:
            site_id = sites[0]['id']
        else:
            site_id = graph('mutation($name:String!,$subnet:Cidr!){createSite(name:$name,subnet:$subnet){id}}',
                            {'name': 'Link', 'subnet': 'fd41:4e49:4d41::/48'}, token)['createSite']['id']
        return graph('mutation($input:CreateDeviceInput!){createDevice(input:$input){id}}',
                     {'input': {'siteId': site_id, 'name': name, 'host': address,
                                'kind': 'server', 'os': 'LINUX', 'tags': [], 'addDefaultCheck': False}}, token)['createDevice']['id']
