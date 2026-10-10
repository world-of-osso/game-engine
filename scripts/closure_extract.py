#!/usr/bin/env python3
"""Publish locally extracted CASC bytes only after authentic content validation."""
from collections import defaultdict
import hashlib
import json
import os
from pathlib import Path
import threading
import shutil
import tempfile

RECEIPT_LOCK = threading.Lock()


def file_hashes(path):
    md5 = hashlib.md5()
    sha256 = hashlib.sha256()
    size = 0
    with path.open('rb') as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b''):
            md5.update(block)
            sha256.update(block)
            size += len(block)
    return size, md5.hexdigest(), sha256.hexdigest()


def read_receipts(index):
    if not index.exists():
        return {}
    rows = {}
    with index.open() as stream:
        for line in stream:
            row = json.loads(line)
            rows[row['path']] = row
    return rows


def summarize_receipts(rows):
    totals = defaultdict(lambda: {'files': 0, 'bytes': 0})
    for row in rows:
        if row['disposition'] == 'extracted':
            totals[row['type']]['files'] += 1
            totals[row['type']]['bytes'] += row['size']
    return dict(sorted(totals.items()))


def runtime_path(data, relative):
    path = Path(relative)
    if path.is_absolute() or '..' in path.parts:
        raise ValueError(f'invalid runtime path {relative}')
    target = data / path
    if not target.resolve().is_relative_to(data.resolve()):
        raise ValueError(f'runtime path escapes data: {relative}')
    return target


def append_receipt(index, row):
    index.parent.mkdir(parents=True, exist_ok=True)
    encoded = (json.dumps(row, sort_keys=True) + '\n').encode()
    fd = os.open(index, os.O_WRONLY | os.O_CREAT | os.O_APPEND, 0o644)
    try:
        if os.write(fd, encoded) != len(encoded):
            raise OSError('short provenance append')
        os.fsync(fd)
    finally:
        os.close(fd)


def publish(stage, data, locations, identity, index, receipts=None):
    size, content, sha256 = file_hashes(stage)
    if size != identity['size']:
        raise ValueError(f'decoded size {size} != expected {identity["size"]}')
    if content != identity['content_key']:
        raise ValueError(f'content key mismatch {content} != {identity["content_key"]}')
    targets = [(relative, runtime_path(data, relative)) for relative in locations]
    result = {'created': 0, 'bytes': 0, 'conflicts': []}
    with RECEIPT_LOCK:
        receipts = read_receipts(index) if receipts is None else receipts
        for relative, target in targets:
            target.parent.mkdir(parents=True, exist_ok=True)
            with tempfile.NamedTemporaryFile(dir=target.parent, prefix='.closure-') as temporary:
                with stage.open('rb') as source:
                    shutil.copyfileobj(source, temporary, 1024 * 1024)
                temporary.flush()
                os.fsync(temporary.fileno())
                try:
                    os.link(temporary.name, target)
                    disposition = 'extracted'
                    result['created'] += 1
                    result['bytes'] += size
                except FileExistsError:
                    if file_hashes(target) != (size, content, sha256):
                        result['conflicts'].append(relative)
                        continue
                    disposition = 'verified_existing'
            previous = receipts.get(relative)
            if previous and all(previous.get(k) == v for k, v in identity.items()) and previous['sha256'] == sha256:
                continue
            row = dict(identity, path=relative, sha256=sha256, disposition=disposition)
            append_receipt(index, row)
            receipts[relative] = row
    return result
