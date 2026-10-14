"""Build identity of authenticated, exported Retail DB2 sources."""
import json
from pathlib import Path


def source_identity(directory):
    receipt = json.loads((Path(directory) / 'receipt.json').read_text())
    identity = receipt['identity']
    if identity['product'] != 'wow':
        raise ValueError(f'{directory}: expected Retail DB2 source')
    return identity


def active_directory(data):
    directory = Path(data) / 'db2/active'
    source_identity(directory)
    return directory
