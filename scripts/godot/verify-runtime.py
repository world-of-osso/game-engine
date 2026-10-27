#!/usr/bin/env python3
"""Verify a pinned Godot runtime before packaging it."""

import argparse
import hashlib
import os
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('runtime', type=Path)
    parser.add_argument('checksum', type=Path)
    args = parser.parse_args()
    try:
        expected = args.checksum.read_text().strip()
        if not os.access(args.runtime, os.X_OK):
            raise ValueError('runtime is not executable')
        with args.runtime.open('rb') as stream:
            actual = hashlib.file_digest(stream, 'sha512').hexdigest()
        if actual != expected:
            raise ValueError('SHA-512 mismatch')
    except (OSError, ValueError) as error:
        parser.exit(1, f'{args.runtime}: {error}\n')
    print(args.runtime)


if __name__ == '__main__':
    main()
