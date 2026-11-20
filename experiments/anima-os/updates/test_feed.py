"""Only authentic, compatible release metadata may authorize downloaded system code."""
import base64
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from feed import FEED, verify


class SignedRelease(unittest.TestCase):
    def test_tampering_wrong_key_and_signed_incompatible_payloads_are_rejected(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            key, public, payload = root/'private.pem', root/'public.pem', root/'manifest.json'
            subprocess.run(['openssl','genpkey','-algorithm','RSA','-pkeyopt','rsa_keygen_bits:2048','-out',str(key)],check=True,capture_output=True)
            subprocess.run(['openssl','pkey','-in',str(key),'-pubout','-out',str(public)],check=True,capture_output=True)
            release = {'format':1,'version':'20261120000000','arch':'x86_64-linux','channel':'preview',
                       'system':'/nix/store/'+'a'*32+'-nixos-system-anima-Preview','sha256':'a'*64,
                       'bytes':1024,'unpackedBytes':4096,'url':FEED.rsplit('/',1)[0]+'/20261120000000/system.nar.xz'}
            def sign(value):
                payload.write_text(json.dumps(value))
                signature=subprocess.check_output(['openssl','dgst','-sha256','-sign',str(key),str(payload)])
                return {'payload':base64.b64encode(payload.read_bytes()).decode(),'signature':base64.b64encode(signature).decode()}
            good=sign(release)
            self.assertEqual(verify(json.dumps(good),public)['system'],release['system'])
            bad=dict(good,payload=base64.b64encode(b'{}').decode())
            with self.assertRaisesRegex(ValueError,'signature'):
                verify(json.dumps(bad),public)
            for field,value in [('arch','aarch64-linux'),('url','https://example.org/system.nar.xz'),('system','/tmp/system'),('bytes',-1)]:
                with self.subTest(field=field),self.assertRaises(ValueError):
                    verify(json.dumps(sign(dict(release,**{field:value}))),public)
            subprocess.run(['openssl','genpkey','-algorithm','RSA','-pkeyopt','rsa_keygen_bits:2048','-out',str(key)],check=True,capture_output=True)
            with self.assertRaisesRegex(ValueError,'signature'):
                verify(json.dumps(sign(release)),public)


if __name__=='__main__':
    unittest.main()
