"""Publish immutable OS artifacts, verify public bytes, then atomically replace the signed feed."""
import argparse
import io
import json
from pathlib import Path
import shlex
import re
import urllib.parse
import tempfile
import boto3
from boto3.s3.transfer import TransferConfig
from botocore.exceptions import ClientError
from feed import FEED, digest, fetch, verify


def client(path):
    values={}
    for line in path.read_text().splitlines():
        line=line.strip().removeprefix('export ')
        if not line or line.startswith('#') or '=' not in line:
            continue
        name,value=line.split('=',1)
        values[name.strip()]=shlex.split(value)[0] if value else ''
    account=values['ACCOUNT_ID']
    endpoint=values.get('S3_API') or (account if account.startswith('https://') else 'https://'+account+'.r2.cloudflarestorage.com')
    parsed=urllib.parse.urlsplit(endpoint)
    if parsed.scheme!='https' or not re.fullmatch(r'[a-f0-9]{32}\.r2\.cloudflarestorage\.com',parsed.netloc):
        raise ValueError('The R2 endpoint configuration is invalid')
    endpoint='https://'+parsed.netloc
    return boto3.client('s3',endpoint_url=endpoint,region_name='auto',
                        aws_access_key_id=values['ACCESS_KEY_ID'],aws_secret_access_key=values['SECRET_KEY'])


def publish(folder, credentials):
    release=verify((folder/'latest.json').read_bytes())
    archive=folder/release['version']/'system.nar.xz'
    if archive.stat().st_size!=release['bytes'] or digest(archive)!=release['sha256']:
        raise ValueError('Local artifact does not match its signed manifest')
    bucket='anima-releases';prefix='os/preview/x86_64-linux/'
    key=prefix+release['version']+'/system.nar.xz'
    storage=client(credentials)
    try:
        storage.head_object(Bucket=bucket,Key=key)
    except ClientError as error:
        if error.response['ResponseMetadata']['HTTPStatusCode']!=404:
            raise
        storage.upload_file(str(archive),bucket,key,ExtraArgs={'ContentType':'application/octet-stream','CacheControl':'public, max-age=31536000, immutable'},
                            Config=TransferConfig(multipart_threshold=64*1024**2,multipart_chunksize=64*1024**2,max_concurrency=3))
    with tempfile.TemporaryDirectory() as temporary:
        downloaded=Path(temporary)/'artifact'
        with downloaded.open('wb') as output:
            fetch(release['url'],output,release['bytes'])
        if digest(downloaded)!=release['sha256']:
            raise ValueError('Public artifact verification failed; feed was not changed')
    storage.put_object(Bucket=bucket,Key=prefix+'latest.json',Body=(folder/'latest.json').read_bytes(),
                       ContentType='application/json',CacheControl='no-cache')
    response=io.BytesIO();fetch(FEED,response,65536)
    if verify(response.getvalue())!=release:
        raise ValueError('Public feed did not return the published release')
    print(json.dumps({'published':release['version'],'feed':FEED,'sha256':release['sha256']}))


if __name__=='__main__':
    parser=argparse.ArgumentParser()
    parser.add_argument('folder',type=Path)
    parser.add_argument('--credentials',type=Path,required=True)
    args=parser.parse_args()
    publish(args.folder,args.credentials)
