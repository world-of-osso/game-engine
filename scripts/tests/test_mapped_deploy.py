import json
import subprocess
import tempfile
import unittest
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import mapped_deploy
import player_bundle


class MappedDeployTest(unittest.TestCase):
    def test_upload_rejects_mutated_source_before_publishing_manifests(self):
        import hashlib
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / 'source'
            source.write_bytes(b'mutated!')
            manifest = {'files': [{'path': 'data/cache/catalog.sqlite', 'size': 8,
                                   'sha256': hashlib.sha256(b'expected').hexdigest()}]}
            rclone = root / 'rclone'
            rclone.write_text('''#!/usr/bin/env python3
import pathlib, sys
args = sys.argv[1:]
if args[0] == "rcat":
    path = pathlib.Path(args[1]); path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(sys.stdin.buffer.read())
elif args[0] == "deletefile":
    pathlib.Path(args[1]).unlink(missing_ok=True)
else:
    raise SystemExit("unexpected publication: " + args[0])
''')
            rclone.chmod(0o755)
            with self.assertRaisesRegex(ValueError, 'data/cache/catalog.sqlite'):
                mapped_deploy.publish({'data/cache/catalog.sqlite': source}, manifest,
                                      {}, root / 'output', root / 'packs',
                                      str(root / 'remote'), str(rclone))
            self.assertFalse((root / 'remote/manifest.json').exists())
            self.assertFalse((root / 'remote/manifest-v2.json').exists())

    def test_dry_run_generates_both_manifests_and_removes_deleted_files(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / 'root'
            root.mkdir()
            subprocess.run(['git', 'init', '-q', str(root)], check=True)
            (root / 'godot').mkdir()
            (root / 'godot/project.godot').write_text('project')
            subprocess.run(['git', '-C', str(root), 'add', 'godot'], check=True)
            for name in player_bundle.DATA_DIRS:
                (root / 'data' / name).mkdir(parents=True)
            (root / 'data/cache/model-asset-index.json').write_text('{"version":1,"assets":[]}')
            asset = root / 'data/textures/example.blp'
            asset.write_bytes(b'asset')
            windows = root / 'windows'
            windows.mkdir()
            for name in ('godot.exe', 'game_engine_godot.dll', 'game-engine-windows-x86_64.exe'):
                (windows / name).write_text(name)
            generated = root / 'generated'
            player_bundle.write_extension_config(generated, 'linux-x86_64')
            output = root / 'out'
            packs = root / 'packs'
            mapped_deploy.build(root, generated, windows, output, packs)
            first = json.loads((output / 'manifest.json').read_text())
            first_v2 = json.loads((packs / 'manifest-v2.json').read_text())
            self.assertEqual([entry['path'] for entry in first['files']],
                             [entry['path'] for entry in first_v2['files']])
            asset.unlink()
            mapped_deploy.build(root, generated, windows, output, packs)
            second = json.loads((output / 'manifest.json').read_text())
            self.assertNotIn('data/textures/example.blp', [entry['path'] for entry in second['files']])
            self.assertEqual((output / 'delete.txt').read_text(), 'data/textures/example.blp\n')


if __name__ == '__main__':
    unittest.main()
