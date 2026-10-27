import hashlib
import importlib.util
import json
import shutil
import sqlite3
import subprocess
import sys
import tempfile
import unittest
from contextlib import closing
from pathlib import Path

spec = importlib.util.spec_from_file_location('player_bundle', Path(__file__).parents[1] / 'player_bundle.py')
bundle = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bundle)


class PlayerBundleTest(unittest.TestCase):
    def test_source_map_matches_assembled_bundle_and_drops_removed_files(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / 'root'
            root.mkdir()
            subprocess.run(['git', 'init', '-q', str(root)], check=True)
            (root / 'godot/scenes').mkdir(parents=True)
            (root / 'godot/project.godot').write_text('project')
            (root / 'godot/scenes/main.tscn').write_text('scene')
            subprocess.run(['git', '-C', str(root), 'add', 'godot'], check=True)
            data = root / 'data'
            for name in bundle.DATA_DIRS:
                (data / name).mkdir(parents=True)
            (data / 'cache/model-asset-index.json').write_text('{"version":1,"assets":[]}')
            asset = data / 'textures/12.blp'
            asset.write_bytes(b'asset')
            artifacts = root / 'windows'
            artifacts.mkdir()
            for name in ('godot.exe', 'game_engine_godot.dll', 'game-engine-windows-x86_64.exe'):
                (artifacts / name).write_text(name)
            staged = root / 'staged'
            bundle.copy_project(root, staged, 'linux-x86_64')
            bundle.copy_data(root, staged)
            bundle.copy_windows_native(artifacts, staged)
            generated = root / 'generated'
            bundle.write_extension_config(generated, 'linux-x86_64')
            mapping = bundle.source_map(root, generated, artifacts)
            expected = {p.relative_to(staged).as_posix(): p.read_bytes()
                        for p in staged.rglob('*') if p.is_file()}
            self.assertEqual({name: path.read_bytes() for name, path in mapping.items()}, expected)
            asset.unlink()
            self.assertNotIn('data/textures/12.blp', bundle.source_map(root, generated, artifacts))

    def test_mapped_manifest_rejects_mutation_and_drops_removed_files(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            catalog = root / 'catalog.sqlite'
            extra = root / 'extra'
            catalog.write_bytes(b'expected')
            extra.write_bytes(b'extra')
            paths = {'data/cache/catalog.sqlite': catalog, 'extra': extra}
            manifest = bundle.mapped_manifest(paths, root / 'digest-cache.json')
            self.assertEqual([entry['path'] for entry in manifest['files']],
                             ['data/cache/catalog.sqlite', 'extra'])
            catalog.write_bytes(b'mutated!')
            with self.assertRaisesRegex(ValueError, 'data/cache/catalog.sqlite'):
                bundle.verify_mapped_manifest(paths, manifest)
            extra.unlink()
            paths.pop('extra')
            refreshed = bundle.mapped_manifest(paths, root / 'digest-cache.json')
            self.assertEqual([entry['path'] for entry in refreshed['files']],
                             ['data/cache/catalog.sqlite'])

    def test_project_only_cli_prepares_import_without_data(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / 'root'
            root.mkdir()
            subprocess.run(['git', 'init', '-q', str(root)], check=True)
            (root / 'godot').mkdir()
            (root / 'godot/project.godot').write_text('project')
            subprocess.run(['git', '-C', str(root), 'add', 'godot'], check=True)
            output = root / 'generated'
            result = subprocess.run([sys.executable, str(Path(bundle.__file__)), '--root', str(root),
                                     '--bundle', str(output), '--platform', 'linux-x86_64',
                                     '--project-only'], capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual((output / 'godot/project.godot').read_text(), 'project')
            self.assertTrue((output / 'godot/.godot/extension_list.cfg').exists())
            self.assertFalse((output / 'data').exists())

    def test_allowlist_excludes_credentials_and_extraction_state(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            data = root / 'data'
            for name in bundle.DATA_DIRS:
                (data / name).mkdir(parents=True)
            for name in ('cache/catalog.sqlite', 'cache/casc/root.bin', 'cache/pre-retail-old/a',
                         'textures/12.blp', 'textures/13.blp.missing', 'cache/a.lock',
                         'auth_token.realm', 'auth_token.private.json', 'diagnostics/run.log', 'Map.csv', 'local-listfile-cache.sqlite',
                         'battle-pet-model-scenes.json',
                         'forever-1.60.1.70205/models/a.m2', 'gametables/classes.txt',
                         'products/wow/dcfc90fffd79ba00406ae46f5f657592/models/1011653.m2',
                         'products/wow/dcfc90fffd79ba00406ae46f5f657592/models/1011653.m2.receipt.json'):
                path = data / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(name)
            index = {'version': 1, 'assets': [{'product': 'wow', 'build_key': 'dcfc90fffd79ba00406ae46f5f657592', 'path': 'products/wow/dcfc90fffd79ba00406ae46f5f657592/models/1011653.m2'}]}
            (data / 'cache/model-asset-index.json').write_text(json.dumps(index))
            expected = {'cache/model-asset-index.json', 'cache/catalog.sqlite', 'textures/12.blp', 'Map.csv', 'battle-pet-model-scenes.json',
                        'local-listfile-cache.sqlite', 'forever-1.60.1.70205/models/a.m2',
                        'gametables/classes.txt',
                        'products/wow/dcfc90fffd79ba00406ae46f5f657592/models/1011653.m2',
                        'products/wow/dcfc90fffd79ba00406ae46f5f657592/models/1011653.m2.receipt.json'}
            selected = list(bundle.data_files(root))
            self.assertEqual({str(p) for p in selected}, expected)
            target = root / 'player'
            bundle.copy_data(root, target)
            self.assertEqual({str(p.relative_to(target / 'data')) for p in (target / 'data').rglob('*') if p.is_file()}, expected)
            for relative in selected:
                self.assertEqual((target / 'data' / relative).read_bytes(), (data / relative).read_bytes())

    def test_linux_data_shares_storage_with_source_through_symlinks(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            data = root / 'data'
            for name in bundle.DATA_DIRS:
                (data / name).mkdir(parents=True)
            (data / 'cache/model-asset-index.json').write_text('{"version":1,"assets":[]}')
            (data / 'textures/12.blp').write_text('direct')
            canonical = root / 'canonical.blp'
            canonical.write_text('linked')
            (data / 'textures/13.blp').symlink_to(canonical)
            target = root / 'player'
            bundle.copy_data(root, target)
            for name, source in (('textures/12.blp', data / 'textures/12.blp'), ('textures/13.blp', canonical)):
                shipped = target / 'data' / name
                self.assertFalse(shipped.is_symlink())
                self.assertEqual(shipped.stat().st_ino, source.stat().st_ino)
            shutil.rmtree(target)
            self.assertEqual(canonical.read_text(), 'linked')

    def test_bundle_selects_current_receipt_and_excludes_old_namespace(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            data = root / 'data'
            for name in bundle.DATA_DIRS:
                (data / name).mkdir(parents=True)
            old = 'products/wow/' + '1' * 32 + '/textures/126862.blp'
            new = 'products/wow/' + '2' * 32 + '/textures/126862.blp'
            for name, raw in [(old, b'old'), (new, b'new')]:
                path = data / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(raw)
            (data / 'cache/model-asset-index.json').write_text(json.dumps({'version': 1, 'assets': [{'product': 'wow', 'build_key': '2' * 32, 'path': new}]}))
            target = root / 'player'
            bundle.copy_data(root, target)
            self.assertEqual((target / 'data' / new).read_bytes(), b'new')
            self.assertFalse((target / 'data' / old).exists())

    def test_mutable_catalog_staging_retains_preparation_snapshot(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            data = root / 'data'
            for name in bundle.DATA_DIRS:
                (data / name).mkdir(parents=True)
            texts = {
                'cache/model-asset-index.json': '{"version":1,"assets":[]}',
                'equipment_transforms.ron': '()',
                'db2/active/Rows.csv': 'ID,Name\n1,prepared\n',
                'gametables/classes.txt': 'prepared',
            }
            for name, content in texts.items():
                path = data / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(content)
            catalogs = ['cache/catalog.sqlite', 'local-listfile-cache.sqlite']
            for name in catalogs:
                with closing(sqlite3.connect(data / name)) as connection, connection:
                    connection.execute('CREATE TABLE names (name TEXT)')
                    connection.execute("INSERT INTO names VALUES ('prepared')")
            target = root / 'player'
            bundle.copy_data(root, target)
            for name in catalogs:
                with closing(sqlite3.connect(data / name)) as connection, connection:
                    connection.execute("UPDATE names SET name = 'changed'")
                with closing(sqlite3.connect(target / 'data' / name)) as connection:
                    names = connection.execute('SELECT name FROM names').fetchall()
                    self.assertEqual(names, [('prepared',)])
            for name, content in texts.items():
                (data / name).write_text('changed')
                self.assertEqual((target / 'data' / name).read_text(), content)

    def test_manifest_check_uses_staged_bytes_and_rejects_later_mutation(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            staged = root / 'player'
            path = staged / 'data/cache/catalog.sqlite'
            path.parent.mkdir(parents=True)
            path.write_bytes(b'expected')
            manifest = {
                'version': 'fixture',
                'files': [{
                    'path': 'data/cache/catalog.sqlite',
                    'size': 8,
                    'sha256': hashlib.sha256(b'expected').hexdigest(),
                }],
            }
            (staged / 'manifest.json').write_text(json.dumps(manifest))
            command = [sys.executable, str(Path(bundle.__file__)), '--root', str(root),
                       '--bundle', str(staged), '--platform', 'linux-x86_64', '--verify-manifest']
            valid = subprocess.run(command, capture_output=True, text=True)
            self.assertEqual(valid.returncode, 0, valid.stderr)
            path.write_bytes(b'mutated!')
            invalid = subprocess.run(command, capture_output=True, text=True)
            self.assertNotEqual(invalid.returncode, 0)
            self.assertIn('SHA256 mismatch', invalid.stderr)
            self.assertIn('data/cache/catalog.sqlite', invalid.stderr)
            path.write_bytes(b'expected')
            manifest['files'][0]['size'] = 9
            (staged / 'manifest.json').write_text(json.dumps(manifest))
            invalid_size = subprocess.run(command, capture_output=True, text=True)
            self.assertNotEqual(invalid_size.returncode, 0)
            self.assertIn('size mismatch', invalid_size.stderr)
            path.unlink()
            missing = subprocess.run(command, capture_output=True, text=True)
            self.assertNotEqual(missing.returncode, 0)
            self.assertIn('catalog.sqlite', missing.stderr)

    def test_manifest_check_limited_to_upload_list_reads_only_listed_files(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            staged = root / 'player'
            entries = []
            for name in ('changed.bin', 'unchanged.bin'):
                (staged / name).parent.mkdir(parents=True, exist_ok=True)
                (staged / name).write_bytes(b'expected')
                entries.append({'path': name, 'size': 8,
                                'sha256': hashlib.sha256(b'expected').hexdigest()})
            (staged / 'manifest.json').write_text(json.dumps({'version': 'fixture', 'files': entries}))
            upload = root / 'upload.txt'
            upload.write_text('changed.bin\n')
            command = [sys.executable, str(Path(bundle.__file__)), '--root', str(root),
                       '--bundle', str(staged), '--platform', 'linux-x86_64', '--verify-manifest',
                       '--only', str(upload)]
            # An unlisted file is not re-read: only uploaded bytes become published content.
            (staged / 'unchanged.bin').write_bytes(b'mutated!')
            valid = subprocess.run(command, capture_output=True, text=True)
            self.assertEqual(valid.returncode, 0, valid.stderr)
            self.assertIn('Verified 1 staged files', valid.stdout)
            (staged / 'changed.bin').write_bytes(b'mutated!')
            invalid = subprocess.run(command, capture_output=True, text=True)
            self.assertNotEqual(invalid.returncode, 0)
            self.assertIn('SHA256 mismatch: changed.bin', invalid.stderr)

    def test_project_keeps_resources_but_excludes_crates_and_tests(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / 'source'
            root.mkdir()
            names = ('godot/Cargo.toml', 'godot/Cargo.lock', 'godot/project.godot',
                     'godot/scenes/main.tscn', 'godot/shaders/terrain.gdshader',
                     'godot/rust/Cargo.toml', 'godot/rust/src/lib.rs',
                     'godot/tests/run.gd', 'godot/game_engine.gdextension')
            for name in names:
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(name)
            subprocess.run(['git', 'init', '-q', str(root)], check=True)
            subprocess.run(['git', '-C', str(root), 'add', '.'], check=True)
            target = Path(directory) / 'player'
            bundle.copy_project(root, target, 'windows-x86_64')
            for name in ('godot/project.godot', 'godot/scenes/main.tscn', 'godot/shaders/terrain.gdshader'):
                self.assertEqual((target / name).read_text(), name)
            self.assertFalse((target / 'godot/rust').exists())
            self.assertFalse((target / 'godot/tests').exists())
            self.assertFalse((target / 'godot/Cargo.toml').exists())
            self.assertEqual((target / 'godot/.godot/extension_list.cfg').read_text(), 'res://game_engine.gdextension\n')

    def test_windows_native_files_join_the_shared_bundle(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            artifacts = root / 'artifacts'
            artifacts.mkdir()
            for name in ('godot.exe', 'game_engine_godot.dll', 'game-engine-windows-x86_64.exe'):
                (artifacts / name).write_bytes(name.encode())
            target = root / 'player'
            bundle.copy_windows_native(artifacts, target)
            for source, destination in (
                ('godot.exe', 'runtime/godot.exe'),
                ('game_engine_godot.dll', 'lib/game_engine_godot.dll'),
                ('game-engine-windows-x86_64.exe', 'game-engine-windows-x86_64'),
            ):
                self.assertEqual((target / destination).read_bytes(), source.encode())

    def test_platform_extension_contract(self):
        for platform, library in [('linux-x86_64', 'linux.x86_64 = "res://../lib/libgame_engine_godot.so"'),
                                  ('windows-x86_64', 'windows.x86_64 = "res://../lib/game_engine_godot.dll"')]:
            with tempfile.TemporaryDirectory() as directory:
                target = Path(directory)
                bundle.write_extension_config(target, platform)
                self.assertIn(library, (target / 'godot/game_engine.gdextension').read_text())


if __name__ == '__main__':
    unittest.main()
