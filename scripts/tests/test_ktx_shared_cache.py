"""Exercise the real compiled build script across concurrent, fresh Cargo OUT_DIRs.

Uses the newest target/debug/build/ktx2-rw-*/build-script-build of this checkout
(KTX_BUILD_SCRIPT overrides it). Run after one helper build has populated the host
cache. Network is disabled.
"""
import concurrent.futures
import hashlib
import os
from pathlib import Path
import subprocess
import tempfile
import unittest


def find_ktx_build_script(root):
    if 'KTX_BUILD_SCRIPT' in os.environ:
        return Path(os.environ['KTX_BUILD_SCRIPT']).resolve()
    # Only a binary compiled after the current build.rs/cache.rs exercises them.
    sources = root / 'vendor/ktx2-rw'
    newest_source = max(
        (sources / name).stat().st_mtime for name in ('build.rs', 'cache.rs')
    )
    scripts = sorted(
        (
            path
            for path in (root / 'target/debug/build').glob('ktx2-rw-*/build-script-build')
            if path.stat().st_mtime >= newest_source
        ),
        key=lambda path: path.stat().st_mtime,
    )
    if not scripts:
        raise AssertionError(
            f'no ktx2-rw build script under {root}/target/debug/build is newer than '
            'vendor/ktx2-rw/build.rs; build the extension once (scripts/depot-build.py) '
            'before this test'
        )
    return scripts[-1]


class SharedKtxCache(unittest.TestCase):
    def test_fresh_out_dirs_reuse_one_library_and_identical_bindings_offline(self):
        root = Path(__file__).resolve().parents[2]
        script = find_ktx_build_script(root)
        vendor = root / 'vendor/ktx2-rw'
        with tempfile.TemporaryDirectory() as temporary:
            directories = [Path(temporary) / str(i) for i in range(3)]
            for directory in directories:
                directory.mkdir()
            environment = os.environ | {
                'TARGET': 'x86_64-unknown-linux-gnu',
                'HOST': 'x86_64-unknown-linux-gnu',
                'CARGO_CFG_TARGET_OS': 'linux',
                'CARGO_CFG_TARGET_ARCH': 'x86_64',
                'CARGO_CFG_TARGET_ENV': 'gnu',
                'CARGO_MANIFEST_DIR': str(vendor),
                'PROFILE': 'debug', 'DEBUG': 'true', 'OPT_LEVEL': '2',
                'NUM_JOBS': '2',
                'HTTP_PROXY': 'http://127.0.0.1:9',
                'HTTPS_PROXY': 'http://127.0.0.1:9',
                'ALL_PROXY': 'http://127.0.0.1:9',
                'NO_PROXY': '',
            }

            def invoke(directory):
                result = subprocess.run(
                    [str(script)], cwd=vendor,
                    env=environment | {'OUT_DIR': str(directory)},
                    capture_output=True, text=True, timeout=45,
                )
                self.assertEqual(result.returncode, 0, result.stderr)
                search_paths = [line for line in result.stdout.splitlines()
                                if line.startswith('cargo:rustc-link-search=native=')]
                self.assertTrue(search_paths, result.stdout)
                bindings = (directory / 'bindings.rs').read_bytes()
                return search_paths[0], hashlib.sha256(bindings).hexdigest()

            with concurrent.futures.ThreadPoolExecutor(max_workers=3) as executor:
                results = list(executor.map(invoke, directories))
            self.assertEqual(len(set(results)), 1)
            library_directory = Path(results[0][0].split('=', 2)[2])
            self.assertTrue((library_directory / 'libktx.a').is_file())
            self.assertFalse(any(library_directory.is_relative_to(d) for d in directories))


if __name__ == '__main__':
    unittest.main()
