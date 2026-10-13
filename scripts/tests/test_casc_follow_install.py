"""Real tiny local archive: authenticated extraction provenance follows .build.info."""
import hashlib
from pathlib import Path
import struct
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import stage_mount_model_assets as stage


class FollowInstallTest(unittest.TestCase):
    def test_patch_records_installed_build_not_authored_db2_build(self):
        with tempfile.TemporaryDirectory() as directory:
            install = Path(directory)
            archive_dir = install / 'Data/data'
            archive_dir.mkdir(parents=True)
            archive, entries = bytearray(), []

            def store(raw):
                blte = b'BLTE\0\0\0\0N' + raw
                key = hashlib.md5(blte).digest()
                payload = bytes(30) + blte
                entries.append(key[:9] + len(archive).to_bytes(5, 'big') + len(payload).to_bytes(4, 'little'))
                archive.extend(payload)
                return hashlib.md5(raw).hexdigest(), key.hex()

            builds = []
            for version, content in [('12.1.0.69933', b'before-patch'), ('12.1.5.70340', b'after-patch')]:
                ck = hashlib.md5(content).digest()
                root = struct.pack('<IIII', 1, 0, 2, 120191) + ck + bytes(8)
                root_ck, root_ek = store(root)
                page = b'\1' + len(root).to_bytes(5, 'big') + bytes.fromhex(root_ck) + bytes.fromhex(root_ek)
                page += bytes(1024 - len(page))
                encoding = struct.pack('>2sBBBHHIIBI', b'EN', 1, 16, 16, 1, 1, 1, 0, 0, 0)
                encoding += bytes.fromhex(root_ck) + hashlib.md5(page).digest() + page
                encoding_ck, encoding_ek = store(encoding)
                config = f'root = {root_ck}\nencoding = {encoding_ck} {encoding_ek}\n'.encode()
                key = hashlib.md5(config).hexdigest()
                config_path = install / f'Data/config/{key[:2]}/{key[2:4]}/{key}'
                config_path.parent.mkdir(parents=True)
                config_path.write_bytes(config)
                builds.append((key, version, ck))
            header = struct.pack('<H6BQ', 7, 0, 0, 4, 5, 9, 30, 0)
            index = struct.pack('<II', len(header), 0) + header + bytes(8)
            payload = b''.join(entries)
            index += struct.pack('<II', len(payload), 0) + payload
            (archive_dir / '0000000001.idx').write_bytes(index)
            (archive_dir / 'data.000').write_bytes(archive)
            for key, version, ck in builds:
                (install / '.build.info').write_text(
                    'Active!DEC:1|Build Key!HEX:16|Version!STRING:0|Product!STRING:0\n'
                    f'1|{key}|{version}|wow\n')
                identity, roots = stage.authenticate_install(install)
                self.assertEqual((identity['build_key'], identity['build']), (key, version))
                self.assertEqual(roots[120191], ck)


if __name__ == '__main__':
    unittest.main()
