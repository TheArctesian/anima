"""Make an offline HTTPS update origin on a disposable fixture disk; never uses host networking."""
from pathlib import Path
import os
import shutil
import subprocess


def prepare(source, output):
    tree=output/'update-origin'
    destination=tree/'os/preview/x86_64-linux'
    destination.mkdir(parents=True)
    for path in source.rglob('*'):
        relative=path.relative_to(source)
        if path.is_dir():
            (destination/relative).mkdir(exist_ok=True)
        else:
            os.link(path,destination/relative)
    subprocess.run(['openssl','req','-x509','-newkey','rsa:2048','-nodes','-days','1',
                    '-subj','/CN=releases.anima.ing','-addext','subjectAltName=DNS:releases.anima.ing',
                    '-keyout',str(tree/'key.pem'),'-out',str(tree/'certificate.pem')],check=True,capture_output=True)
    (tree/'serve.py').write_text('''from http.server import ThreadingHTTPServer,SimpleHTTPRequestHandler
import os,ssl
os.chdir('/run/update-origin')
server=ThreadingHTTPServer(('127.0.0.1',443),SimpleHTTPRequestHandler)
context=ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
context.load_cert_chain('certificate.pem','key.pem')
server.socket=context.wrap_socket(server.socket,server_side=True)
server.serve_forever()
''')
    disk=output/'update-origin.raw'
    size=sum(path.stat().st_size for path in tree.rglob('*') if path.is_file())+512*1024**2
    with disk.open('wb') as stream:
        stream.truncate(size)
    subprocess.run(['mkfs.ext4','-q','-F','-L','UPDATEFIX','-d',str(tree),str(disk)],check=True)
    return disk


def attach(machine, output, disk):
    from gui_test import qmp
    qmp(output,'blockdev-add',{'node-name':'updates','driver':'raw','read-only':True,'file':{'driver':'file','filename':str(disk)}})
    qmp(output,'device_add',{'driver':'virtio-blk-pci','drive':'updates','id':'update-fixture','bus':'update-port'})
    machine.command('udevadm settle; mkdir -p /run/update-origin; mount -o ro /dev/disk/by-label/UPDATEFIX /run/update-origin')
    machine.command("printf '127.0.0.1 localhost releases.anima.ing\\n' > /tmp/update-hosts; mount --bind /tmp/update-hosts /etc/hosts")
    machine.command("python=$(nix-store -qR /run/current-system | grep -E '/[^/]+-python3-[0-9.]+$' | head -n1); systemd-run --unit=anima-update-fixture --collect $python/bin/python3 /run/update-origin/serve.py; timeout 15 sh -c 'until ss -ltn | grep -q 127.0.0.1:443; do sleep 1; done'")
