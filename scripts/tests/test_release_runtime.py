import hashlib
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


class ReleaseRuntimeTest(unittest.TestCase):
    def test_runtime_checksum_accepts_exact_binary_and_rejects_mutation(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            runtime = root / 'godot-template'
            checksum = root / 'runtime.sha512'
            runtime.write_bytes(b'template')
            runtime.chmod(0o755)
            checksum.write_text(hashlib.sha512(b'template').hexdigest() + '\n')
            command = [sys.executable, str(Path(__file__).parents[1] / 'godot/verify-runtime.py'),
                       str(runtime), str(checksum)]
            good = subprocess.run(command, text=True, capture_output=True)
            self.assertEqual(good.returncode, 0, good.stderr)
            self.assertEqual(good.stdout.strip(), str(runtime))
            runtime.write_bytes(b'mutated!')
            bad = subprocess.run(command, text=True, capture_output=True)
            self.assertNotEqual(bad.returncode, 0)
            self.assertIn('godot-template', bad.stderr)


if __name__ == '__main__':
    unittest.main()
