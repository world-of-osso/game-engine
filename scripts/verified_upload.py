#!/usr/bin/env python3
"""Upload mapped files through temporary objects, promoting only verified bytes."""

import hashlib
import subprocess
import uuid


def upload_file(source, destination, entry, rclone='rclone'):
    temporary = f"{destination.rstrip('/')}/.deploy-temporary/{uuid.uuid4().hex}"
    target = f"{destination.rstrip('/')}/{entry['path']}"
    headers = ['--header-upload', 'Cache-Control: no-cache']
    process = subprocess.Popen([rclone, 'rcat', temporary, *headers], stdin=subprocess.PIPE)
    digest = hashlib.sha256()
    size = 0
    try:
        with source.open('rb') as stream, process.stdin as output:
            while block := stream.read(1024 * 1024):
                output.write(block)
                digest.update(block)
                size += len(block)
        if process.wait() != 0:
            raise RuntimeError(f"Upload failed: {entry['path']}")
        if size != entry['size'] or digest.hexdigest() != entry['sha256']:
            raise ValueError(f"Uploaded bytes do not match manifest: {entry['path']}")
        subprocess.run([rclone, 'moveto', temporary, target, *headers], check=True)
    except BaseException:
        if process.poll() is None:
            process.kill()
            process.wait()
        subprocess.run([rclone, 'deletefile', temporary], check=True)
        raise
