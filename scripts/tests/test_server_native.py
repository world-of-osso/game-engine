"""Native server adapters execute Cargo in the checkout with its real data."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

SCRIPTS = Path(__file__).resolve().parents[1]


class ServerNativeTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.base = Path(self.tmp.name)
        self.root = self.base / "game-server"
        self.root.mkdir()
        (self.root / "scripts").mkdir()
        (self.base / "game-engine").mkdir()
        (self.base / "game-engine/scripts").symlink_to(SCRIPTS, target_is_directory=True)
        (self.root / "data").mkdir()
        (self.root / "data/world.db").write_text("real checkout rows")
        self.bin = self.base / "bin"
        self.bin.mkdir()
        cargo = self.bin / "cargo"
        cargo.write_text('''#!/usr/bin/env python3
import json, os, sys
from pathlib import Path
Path(os.environ['RECORD']).write_text(json.dumps(dict(args=sys.argv[1:], cwd=str(Path.cwd()), target=os.environ['CARGO_TARGET_DIR'], jobs=os.environ.get('CARGO_BUILD_JOBS'), data=Path('data/world.db').read_text())))
if os.environ.get('EXPECT_PROTOCOL'):
    args = sys.argv[1:]
    project = Path(args[args.index('--manifest-path') + 1]).parent if '--manifest-path' in args else Path.cwd()
    assert (project.parent / 'shared-protocol/value').read_text() == os.environ['EXPECT_PROTOCOL']
print('test result: ok. 1 passed; 0 failed; 0 ignored')
raise SystemExit(int(os.environ.get('CARGO_STATUS', '0')))
''')
        cargo.chmod(0o755)
        self.record = self.base / "record.json"
        self.env = dict(os.environ, HOME=str(self.base), XDG_CACHE_HOME=str(self.base / "cache"), PATH=str(self.bin) + os.pathsep + os.environ['PATH'], RECORD=str(self.record), CARGO_BUILD_JOBS="8")
        self.env.pop('BUILD_HOST_SCRIPTS', None)
        for name in list(self.env):
            if name.startswith('DEPOT_SIBLING_'):
                del self.env[name]
        setting = self.base / '.config/game-engine/build-host'
        setting.parent.mkdir(parents=True)
        setting.write_text('native\n')

    def invoke(self, adapter, *args):
        script = SCRIPTS / 'desktop-server-build.py' if adapter == 'export' else self.root / 'scripts/build-host.py'
        return subprocess.run([sys.executable, str(script), '--root', str(self.root), *args], env=self.env, capture_output=True, text=True)

    def test_saved_native_tests_forward_arguments_and_exit_status(self):
        self.env['CARGO_STATUS'] = '7'
        for adapter in ('export',):
            with self.subTest(adapter=adapter):
                result = self.invoke(adapter, '--test', '-p', 'server', 'fixture', '--', '--nocapture')
                self.assertEqual(result.returncode, 7, result.stderr)
                observed = json.loads(self.record.read_text())
                self.assertEqual(observed['args'], ['test', '--locked', '-p', 'server', 'fixture', '--', '--nocapture'])
                self.assertEqual(observed['cwd'], str(self.root))
                self.assertEqual(observed['target'], str(self.root / 'target'))
                self.assertIsNone(observed['jobs'])
                self.assertEqual(observed['data'], 'real checkout rows')

    def test_explicit_native_build_selects_one_binary(self):
        for adapter in ('export',):
            with self.subTest(adapter=adapter):
                result = self.invoke(adapter, '--build-host', 'native', '--bin', 'game-cli')
                self.assertEqual(result.returncode, 0, result.stderr)
                observed = json.loads(self.record.read_text())
                self.assertEqual(observed['args'], ['build', '--locked', '-p', 'client', '--bin', 'game-cli'])
                self.assertNotIn('-j8', observed['args'])

    def test_native_uses_overridden_protocol_beside_manifest(self):
        protocol = self.base / 'protocol-branch'
        protocol.mkdir()
        (protocol / 'value').write_text('branch protocol')
        self.env.update(DEPOT_SIBLING_SHARED_PROTOCOL=str(protocol), EXPECT_PROTOCOL='branch protocol')
        result = self.invoke('export', '--build-host', 'native', '--test', '-p', 'server')
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_real_cargo_resolves_protocol_override_and_checkout_data(self):
        (self.root / 'Cargo.toml').write_text(
            '[package]\nname="server"\nversion="0.1.0"\nedition="2024"\n'
            '[dependencies]\nshared-protocol={path="../shared-protocol"}\n'
        )
        (self.root / 'Cargo.lock').write_text(
            'version = 4\n[[package]]\nname = "server"\nversion = "0.1.0"\n'
            'dependencies = ["shared-protocol"]\n'
            '[[package]]\nname = "shared-protocol"\nversion = "0.1.0"\n'
        )
        (self.root / 'src').mkdir()
        (self.root / 'src/lib.rs').write_text(
            '#[test] fn native_paths() { assert_eq!(shared_protocol::VALUE, 42); '
            'assert_eq!(std::fs::read_to_string("data/world.db").unwrap(), '
            '"real checkout rows"); }\n'
        )
        protocol = self.base / 'protocol-branch'
        (protocol / 'src').mkdir(parents=True)
        (protocol / 'Cargo.toml').write_text(
            '[package]\nname="shared-protocol"\nversion="0.1.0"\nedition="2024"\n'
        )
        (protocol / 'src/lib.rs').write_text('pub const VALUE: u32 = 42;\n')
        self.env.update(DEPOT_SIBLING_SHARED_PROTOCOL=str(protocol))
        self.env['PATH'] = os.environ['PATH']
        result = self.invoke('export', '--build-host', 'native', '--test')
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn('1 passed; 0 failed', result.stdout)
        self.assertTrue((self.root / 'target/debug/deps').is_dir())

    def test_native_release_fails_before_cargo(self):
        for adapter in ('export',):
            with self.subTest(adapter=adapter):
                result = self.invoke(adapter, '--build-host', 'native', '--release')
                self.assertEqual(result.returncode, 1, result.stderr)
                self.assertIn('bookworm container', result.stderr)
                self.assertFalse(self.record.exists())


if __name__ == '__main__':
    unittest.main()
