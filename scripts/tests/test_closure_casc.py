"""Concrete CASC metadata fixtures, with no extraction or network."""
from pathlib import Path
import hashlib
import struct
import sys
import unittest
import zlib

sys.path.insert(0, str(Path(__file__).parents[1]))


def idx_entry(key, archive, offset, size):
    return key[:9] + ((archive << 30) | offset).to_bytes(5, 'big') + struct.pack('<I', size)


class CascTests(unittest.TestCase):
    def test_idx_latest_update_removes_sorted_entry(self):
        from closure_casc import parse_idx
        a, b = b'a' * 16, b'b' * 16
        header = struct.pack('<H6BQ', 7, 0, 0, 4, 5, 9, 30, 0x40000000)
        entry = idx_entry(a, 2, 100, 40)
        data = struct.pack('<II', 16, 0) + header + bytes(8) + struct.pack('<II', len(entry), 0) + entry
        data += bytes(65536 - len(data))
        update = struct.pack('<I', 0x80000001) + idx_entry(a, 2, 100, 40) + bytes([3, 0])
        update += struct.pack('<I', 0x80000002) + idx_entry(b, 3, 200, 60) + bytes([0, 0])
        data += update + bytes(512 - len(update))
        self.assertEqual(parse_idx(data), {b[:9]: (3, 200, 60)})

    def test_root_delta_locale_and_last_variant(self):
        from closure_casc import root_keys
        def block(locale, ids, keys):
            deltas = [ids[0]] + [b-a-1 for a, b in zip(ids, ids[1:])]
            return struct.pack('<4IB', len(ids), locale, 0x10000000, 0, 0) + struct.pack('<'+'I'*len(ids), *deltas) + b''.join(keys)
        data = b'TSFM' + struct.pack('<5I', 24, 2, 4, 0, 0)
        data += block(2, [10, 12], [b'a'*16, b'b'*16])
        data += block(4, [10], [b'x'*16]) + block(2, [12], [b'c'*16])
        self.assertEqual(root_keys(data, {10, 12, 99}), {10: b'a'*16, 12: b'c'*16})
        with self.assertRaises(ValueError):
            root_keys(data[:-1], {10})

    def test_encoding_preserves_all_keys_and_content_size(self):
        from closure_casc import encoding_keys
        ck, a, b = b'c'*16, b'a'*16, b'b'*16
        entry = bytes([2]) + (12345).to_bytes(5, 'big') + ck + a + b
        page = entry + bytes(1024-len(entry))
        data = struct.pack('>2sBBBHHIIBI', b'EN', 1, 16, 16, 1, 1, 1, 0, 0, 1)
        data += b'\0' + ck + hashlib.md5(page).digest() + page
        self.assertEqual(encoding_keys(data, {ck}), {ck: (12345, [a, b])})

    def test_blte_chunked_zlib_and_checksum(self):
        from closure_casc import decode_blte
        raw = b'root-metadata'
        compressed = b'Z' + zlib.compress(raw)
        data = b'BLTE' + struct.pack('>I', 36) + bytes([15, 0, 0, 1])
        data += struct.pack('>II', len(compressed), len(raw)) + hashlib.md5(compressed).digest() + compressed
        self.assertEqual(decode_blte(data), raw)
        with self.assertRaises(ValueError):
            decode_blte(data[:-1] + bytes([data[-1] ^ 1]))

    def test_chain_distinguishes_absent_root_encoding_and_idx(self):
        from closure_casc import resolve_missing
        roots = {1:b'a'*16, 2:b'b'*16, 3:b'c'*16, 4:b'd'*16}
        enc = {b'b'*16:(40,[b'x'*16]), b'c'*16:(60,[b'y'*16,b'z'*16]), b'd'*16:(70,[b'w'*16])}
        idx = {b'z'*9:(0,10,50), b'w'*9:(0,1000,100)}
        result = resolve_missing({1,2,3,4,5}, roots, enc, idx, {0:500})
        self.assertEqual(result['counts'], {'root_missing':1, 'encoding_missing':1, 'idx_missing':1, 'archive_missing':1, 'local_index_present':1})
        self.assertEqual(result['local_content_bytes'], 60)
        self.assertEqual(result['files'][2]['encoding_key'], (b'z'*16).hex())


if __name__ == '__main__':
    unittest.main()
