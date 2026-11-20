"""Disposable VM-only hub fixture: observe the real packaged daemon's storage announcements."""
import base64
from http.server import BaseHTTPRequestHandler
import json
from pathlib import Path
import socketserver

ROOT=Path('/var/lib/anima-daemon')


class Hub(BaseHTTPRequestHandler):
    def log_message(self,*args):
        pass

    def do_POST(self):
        body=json.loads(self.rfile.read(int(self.headers['Content-Length'])))
        if self.path=='/daemon/v1/enrol':
            payload=json.dumps({'device':'00000000-0000-0000-0000-0000000000d1','deviceName':'fixture',
                                'token':'anima_daemon_'+base64.urlsafe_b64encode(bytes([7])*32).decode().rstrip('='),
                                'expiresAt':'2027-11-08T10:00:00Z'}).encode()
            self.send_response(200);self.send_header('Content-Length',str(len(payload)));self.end_headers();self.wfile.write(payload)
        elif self.path=='/daemon/v1/push':
            with (ROOT/'fixture-pushes.jsonl').open('a') as output:
                output.write(json.dumps(body)+'\n')
            self.send_response(204);self.end_headers()
        else:
            self.send_response(404);self.end_headers()


socket=ROOT/'link.sock'
socket.unlink(missing_ok=True)
with socketserver.UnixStreamServer(str(socket),Hub) as server:
    server.serve_forever()
