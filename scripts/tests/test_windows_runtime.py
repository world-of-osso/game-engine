import hashlib
import importlib.util
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

scripts = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(scripts))
worker_spec = importlib.util.spec_from_file_location('windows_worker', scripts / 'windows-worker.py')
worker = importlib.util.module_from_spec(worker_spec)
worker_spec.loader.exec_module(worker)
release_spec = importlib.util.spec_from_file_location('windows_release', scripts / 'windows-release.py')
release = importlib.util.module_from_spec(release_spec)
release_spec.loader.exec_module(release)


class WindowsRuntimeTest(unittest.TestCase):
    def test_build_ships_template_and_checksum_but_retains_editor(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / 'godot-source'
            source.mkdir()
            engine = root / 'engine'
            (engine / 'scripts/godot').mkdir(parents=True)
            (engine / 'scripts/godot/pr123546-classdb-default-values-race.patch').write_text('patch')
            bin_dir = root / 'tools'
            bin_dir.mkdir()
            git = bin_dir / 'git'
            git.write_text('#!/usr/bin/python3\nimport sys\nif "rev-parse" in sys.argv: print("' + worker.GODOT_COMMIT + '")\n')
            git.chmod(0o755)
            uv = bin_dir / 'uv'
            uv.write_text('''#!/usr/bin/python3
import pathlib, sys
out = pathlib.Path.cwd() / 'bin'
out.mkdir(exist_ok=True)
if 'target=editor' in sys.argv:
    (out / 'godot.windows.editor.x86_64.exe').write_bytes(b'editor')
if 'target=template_release' in sys.argv:
    payload = b'template' if 'disable_path_overrides=no' in sys.argv else b'path-disabled'
    (out / 'godot.windows.template_release.x86_64.exe').write_bytes(payload)
''')
            uv.chmod(0o755)
            with patch.dict(os.environ, {'PATH': str(bin_dir) + os.pathsep + os.environ['PATH']}):
                worker.build_runtime(root, uv, engine)
            runtime = root / 'artifacts/godot.exe'
            self.assertEqual(runtime.read_bytes(), b'template')
            self.assertEqual((source / 'bin/godot.windows.editor.x86_64.exe').read_bytes(), b'editor')
            self.assertEqual((root / 'artifacts/godot.sha512').read_text().strip(), hashlib.sha512(b'template').hexdigest())

    def test_import_uses_editor_not_shipped_template(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'player/godot/.godot/editor').mkdir(parents=True)
            (root / 'player/runtime').mkdir()
            (root / 'godot-source/bin').mkdir(parents=True)
            for path, marker in [(root / 'godot-source/bin/godot.windows.editor.x86_64.exe', 'editor'),
                                 (root / 'player/runtime/godot.exe', 'template')]:
                path.write_text('#!/usr/bin/python3\nfrom pathlib import Path\nPath(' + repr(str(root / marker)) + ').write_text("ran")\n')
                path.chmod(0o755)
            with patch.object(worker, 'configure'), patch.object(sys, 'argv', ['worker', '--root', str(root), '--phase', 'import']), patch.dict(os.environ, {}):
                worker.main()
            self.assertTrue((root / 'editor').exists())
            self.assertFalse((root / 'template').exists())
            self.assertFalse((root / 'player/godot/.godot/editor').exists())

    def test_fetched_runtime_checksum_is_required(self):
        with tempfile.TemporaryDirectory() as directory:
            artifacts = Path(directory)
            (artifacts / 'godot.exe').write_bytes(b'template')
            (artifacts / 'godot.sha512').write_text(hashlib.sha512(b'template').hexdigest() + '\n')
            release.verify_runtime_checksum(artifacts)
            (artifacts / 'godot.exe').write_bytes(b'mutated!')
            with self.assertRaisesRegex(ValueError, 'godot.exe'):
                release.verify_runtime_checksum(artifacts)


if __name__ == '__main__':
    unittest.main()
