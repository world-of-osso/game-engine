"""Observe the Cargo process boundary without building crates."""
import importlib.util
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
spec = importlib.util.spec_from_file_location('depot_build', SCRIPTS / 'depot-build.py')
helper = importlib.util.module_from_spec(spec)
spec.loader.exec_module(helper)


class NativeDevEnvironment(unittest.TestCase):
    def test_native_cargo_uses_host_jobs_and_mold_from_unrelated_cwd(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            binary = root / 'cargo'
            binary.write_text(
                '#!/usr/bin/env python3\n'
                'import json, os\n'
                'from pathlib import Path\n'
                'Path(os.environ["OBSERVATION"]).write_text(json.dumps({"cwd": os.getcwd(), "environment": dict(os.environ)}))\n'
            )
            binary.chmod(0o755)
            observation = root / 'observed.json'
            environment = {
                'PATH': str(root) + ':' + os.environ['PATH'],
                'OBSERVATION': str(observation),
                'CARGO_BUILD_JOBS': '99',
            }
            with patch.dict(os.environ, environment, clear=True):
                self.assertEqual(helper.native_cargo(root, 'build', []), 0)
            seen = json.loads(observation.read_text())
            child = seen['environment']
            self.assertEqual(seen['cwd'], str(root))
            self.assertEqual(child['CARGO_BUILD_JOBS'], str(len(os.sched_getaffinity(0))))
            self.assertEqual(child['CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER'], 'clang')
            self.assertEqual(child['CARGO_ENCODED_RUSTFLAGS'].split('\x1f'),
                             ['-C', 'link-arg=-fuse-ld=mold', '-C', 'link-arg=-Wl,--thread-count=4'])
            self.assertEqual(child['CARGO_TARGET_DIR'], str(root / 'target'))


if __name__ == '__main__':
    unittest.main()
