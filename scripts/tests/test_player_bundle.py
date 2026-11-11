import hashlib
import importlib.util
import json
import os
import re
import shutil
import sqlite3
import subprocess
import sys
import tempfile
import unittest
from contextlib import closing
from pathlib import Path


def client_read_files(repo):
    literal_path = re.compile(
        r'(?:self\.)?data_root\s*\.\s*join\(\s*"([^"]+)"'
        r'|resolve_data_path\(\s*"([^"]+)"'
    )
    file_suffixes = {
        '.adt', '.blp', '.blob', '.csv', '.db2', '.json', '.m2', '.mp3',
        '.ogg', '.ron', '.skin', '.skel', '.sqlite', '.txt', '.wav', '.wdt', '.wmo',
    }
    files = set()
    unsupported = set()
    for source in (repo / 'godot').rglob('*.rs'):
        if {'tests', 'target', 'vendor'} & set(source.parts) or source.name.endswith('_tests.rs'):
            continue
        for match in literal_path.finditer(source.read_text(errors='replace')):
            value = next(group for group in match.groups() if group is not None)
            relative = Path(value)
            if relative.is_absolute() or '..' in relative.parts or not relative.parts:
                continue
            if relative.parts[0] in bundle.DATA_DIRS:
                if relative.parts[0] == 'products':
                    continue
                if relative.suffix.lower() in file_suffixes:
                    files.add(relative)
            elif len(relative.parts) == 1 and any(
                relative.match(pattern) for pattern in bundle.DATA_FILE_GLOBS
            ):
                files.add(relative)
            else:
                unsupported.add(value)
    if unsupported:
        raise AssertionError(f'Client data reads outside bundle policy: {sorted(unsupported)}')
    return files


def write_empty_ui_db2(data):
    tables = {
        'UiMap': 'ID,System\n',
        'UiMapXMapArt': 'PhaseID,UiMapArtID,UiMapID\n',
        'UiMapArt': 'ID,HighlightFileDataID,UiMapArtStyleID\n',
        'UiMapArtStyleLayer': 'UiMapArtStyleID,LayerIndex\n',
        'UiMapArtTile': 'UiMapArtID,LayerIndex,FileDataID\n',
        'WorldMapOverlay': 'ID,UiMapArtID\n',
        'WorldMapOverlayTile': 'WorldMapOverlayID,LayerIndex,FileDataID\n',
        'UiTextureAtlasMember': 'ID,UiTextureAtlasID\n',
        'UiTextureAtlas': 'ID,FileDataID\n',
    }
    directory = data / 'db2/active'
    directory.mkdir(parents=True, exist_ok=True)
    for name, content in tables.items():
        (directory / f'{name}.csv').write_text(content)
    forever_atlas = data / 'db2/1.60.1.69913'
    forever_atlas.mkdir(parents=True, exist_ok=True)
    (forever_atlas / 'UiTextureAtlasMember.csv').write_text('ID,UiTextureAtlasID\n')
    (forever_atlas / 'UiTextureAtlas.csv').write_text(
        'ID,FileDataID,UiTextureAtlasSetID\n'
    )


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
            write_empty_ui_db2(data)
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

    def test_manifest_captures_catalog_mutation_during_other_source_hashing(self):
        import threading
        from unittest.mock import patch
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            catalog = root / 'catalog.sqlite'
            other = root / 'other.blp'
            catalog.write_bytes(b'before')
            other.write_bytes(b'payload')
            started, proceed = threading.Event(), threading.Event()
            digest_file = hashlib.file_digest
            def synchronized_digest(stream, algorithm):
                if Path(stream.name) == other:
                    started.set()
                    if not proceed.wait(5):
                        raise TimeoutError('test did not release hash')
                return digest_file(stream, algorithm)
            result = []
            with patch.object(bundle.hashlib, 'file_digest', side_effect=synchronized_digest):
                worker = threading.Thread(target=lambda: result.append(bundle.mapped_manifest(
                    {'data/cache/catalog.sqlite': catalog, 'data/textures/other.blp': other},
                    root / 'cache.json')))
                worker.start()
                self.assertTrue(started.wait(5))
                catalog.write_bytes(b'after!')
                proceed.set()
                worker.join(5)
                self.assertFalse(worker.is_alive())
            entry = next(item for item in result[0]['files'] if item['path'].endswith('catalog.sqlite'))
            self.assertEqual(entry['sha256'], hashlib.sha256(b'after!').hexdigest())

    def test_sqlite_snapshot_remains_consistent_after_source_write(self):
        import importlib
        sys.path.insert(0, str(Path(bundle.__file__).parent))
        packs = importlib.import_module('launcher_packs')
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / 'catalog.sqlite'
            with closing(sqlite3.connect(source)) as database:
                database.execute('PRAGMA journal_mode=WAL')
                database.execute('CREATE TABLE names (name TEXT)')
                database.execute("INSERT INTO names VALUES ('before')")
                database.commit()
                paths = {'data/cache/catalog.sqlite': source}
                selected = bundle.snapshot_sqlite_sources(paths, root / 'snapshots')
                manifest = bundle.mapped_manifest(selected, root / 'digest-cache.json')
                database.execute("INSERT INTO names VALUES ('after')")
                database.commit()
                with closing(sqlite3.connect(selected['data/cache/catalog.sqlite'])) as snapshot:
                    self.assertEqual(snapshot.execute('SELECT name FROM names').fetchall(), [('before',)])
                bundle.verify_mapped_manifest(selected, manifest)
                packed = packs.build_packs(root, root / 'packs', manifest, source_map=selected)
                chunks = packed['files'][0]['chunks']
                payload = b''.join((root / 'packs/packs' / item['pack']).read_bytes()[item['offset']:item['offset'] + item['size']] for item in chunks)
                self.assertEqual(payload, selected['data/cache/catalog.sqlite'].read_bytes())
                with closing(sqlite3.connect(selected['data/cache/catalog.sqlite'])) as snapshot, snapshot:
                    snapshot.execute("INSERT INTO names VALUES ('changed')")
                with self.assertRaisesRegex(ValueError, 'data/cache/catalog.sqlite'):
                    bundle.verify_mapped_manifest(selected, manifest)

    def test_index_selection_rejects_source_changes_and_new_fragments(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cache = root / 'data/cache'
            cache.mkdir(parents=True)
            index = cache / 'model-asset-index.json'
            index.write_text('{"version":1,"assets":[]}')
            state = bundle.model_index_state(root)
            index.write_text('{"version":1,"assets":[1]}')
            with self.assertRaisesRegex(ValueError, 'model-asset-index.json'):
                bundle.verify_model_index_state(root, state)
            state = bundle.model_index_state(root)
            (cache / 'model-asset-index-new.json').write_text('{"version":1,"assets":[]}')
            with self.assertRaisesRegex(ValueError, 'model-asset-index-new.json'):
                bundle.verify_model_index_state(root, state)

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

    def test_literal_client_data_root_reads_are_selected(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            data = root / 'data'
            for name in bundle.DATA_DIRS:
                (data / name).mkdir(parents=True)
            write_empty_ui_db2(data)
            (data / 'cache/model-asset-index.json').write_text(
                '{"version":1,"assets":[]}'
            )
            client_files = client_read_files(Path(__file__).parents[2])
            for relative in client_files:
                path = data / relative
                if relative != Path('cache/model-asset-index.json'):
                    path.parent.mkdir(parents=True, exist_ok=True)
                    path.touch()

            selected = set(bundle.data_files(root))
            self.assertTrue(client_files)
            self.assertEqual(client_files - selected, set())

    def test_allowlist_excludes_credentials_and_extraction_state(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            data = root / 'data'
            for name in bundle.DATA_DIRS:
                (data / name).mkdir(parents=True)
            write_empty_ui_db2(data)
            for name in ('cache/catalog.sqlite', 'cache/casc/root.bin', 'cache/pre-retail-old/a',
                         'textures/12.blp', 'textures/13.blp.missing', 'cache/a.lock',
                         'auth_token.realm', 'auth_token.private.json', 'diagnostics/run.log', 'Map.csv', 'local-listfile-cache.sqlite',
                         'battle-pet-model-scenes.json', 'battle-pets.json',
                         'forever-1.60.1.70205/models/a.m2', 'gametables/classes.txt',
                         'products/wow/dcfc90fffd79ba00406ae46f5f657592/models/1011653.m2',
                         'products/wow/dcfc90fffd79ba00406ae46f5f657592/models/1011653.m2.receipt.json'):
                path = data / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(name)
            index = {'version': 1, 'assets': [{'product': 'wow', 'build_key': 'dcfc90fffd79ba00406ae46f5f657592', 'path': 'products/wow/dcfc90fffd79ba00406ae46f5f657592/models/1011653.m2'}]}
            (data / 'cache/model-asset-index.json').write_text(json.dumps(index))
            expected = {'cache/model-asset-index.json', 'cache/catalog.sqlite', 'textures/12.blp', 'Map.csv', 'battle-pet-model-scenes.json', 'battle-pets.json',
                        'local-listfile-cache.sqlite', *{f'db2/active/{name}.csv' for name in (
                            'UiMap', 'UiMapXMapArt', 'UiMapArt', 'UiMapArtStyleLayer',
                            'UiMapArtTile', 'WorldMapOverlay', 'WorldMapOverlayTile',
                            'UiTextureAtlasMember', 'UiTextureAtlas',
                        )},
                        'db2/1.60.1.69913/UiTextureAtlasMember.csv',
                        'db2/1.60.1.69913/UiTextureAtlas.csv',
                        'forever-1.60.1.70205/models/a.m2',
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

    def test_staged_ui_texture_closure_uses_map_systems_and_both_atlas_sets(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            root.mkdir(exist_ok=True)
            subprocess.run(['git', 'init', '-q', str(root)], check=True)
            (root / 'godot').mkdir()
            (root / 'godot/project.godot').write_text('project')
            subprocess.run(['git', '-C', str(root), 'add', 'godot'], check=True)
            data = root / 'data'
            for name in bundle.DATA_DIRS:
                (data / name).mkdir(parents=True)
            write_empty_ui_db2(data)
            forever_tables = data / 'db2/1.60.1.69913'
            forever_tables.mkdir(parents=True, exist_ok=True)
            (forever_tables / 'UiTextureAtlasMember.csv').write_text(
                'ID,UiTextureAtlasID\n1,4100\n2,4101\n'
            )
            (forever_tables / 'UiTextureAtlas.csv').write_text(
                'ID,FileDataID,UiTextureAtlasSetID\n4100,6855432,1\n4101,999998,2\n'
            )
            (data / 'cache/model-asset-index.json').write_text('{"version":1,"assets":[]}')
            (data / 'battle-pets.json').write_text('[]\n')

            tables = data / 'db2/active'
            rows = {
                'UiMap': 'ID,System\n862,0\n875,0\n1165,0\n2401,2\n2402,1\n2403,3\n',
                'UiMapXMapArt': 'PhaseID,UiMapArtID,UiMapID\n0,887,862\n0,900,875\n0,1143,1165\n0,2056,2401\n0,999,2402\n0,998,2403\n1,1000,1165\n',
                'UiMapArt': 'ID,HighlightFileDataID,UiMapArtStyleID\n887,2168041,5\n900,2168254,5\n1143,0,5\n2056,0,106\n998,777776,7\n999,777777,7\n1000,777778,7\n',
                'UiMapArtStyleLayer': 'UiMapArtStyleID,LayerIndex\n5,0\n5,1\n7,0\n106,0\n',
                'UiMapArtTile': 'UiMapArtID,LayerIndex,FileDataID\n887,0,1599407\n900,0,1603193\n1143,0,2133758\n2056,0,7438338\n1143,1,999999\n998,0,777779\n999,0,777780\n1000,0,777781\n',
                'WorldMapOverlay': 'ID,UiMapArtID\n80,887\n81,2056\n82,998\n83,999\n',
                'WorldMapOverlayTile': 'WorldMapOverlayID,LayerIndex,FileDataID\n80,0,2034162\n81,0,0\n80,1,777782\n82,0,777783\n83,0,777784\n',
                'UiTextureAtlasMember': 'ID,UiTextureAtlasID\n1,3215\n2,3963\n',
                'UiTextureAtlas': 'ID,FileDataID\n3215,6875169\n3963,8040088\n9999,555000\n',
            }
            for name, content in rows.items():
                (tables / f'{name}.csv').write_text(content)

            expected_fdids = {
                1599407, 1603193, 2133758, 7438338, 2034162,
                2168041, 2168254, 6875169, 8040088, 6855432, 999998,
            }
            self.assertEqual(bundle.ui_texture_fdids(root), expected_fdids)
            selected = set(bundle.data_files(root))
            for fdid in expected_fdids:
                self.assertIn(Path(f'textures/{fdid}.blp'), selected)

            extractor = root / 'casc-local-fixture'
            extractor.write_text(
                f'#!{sys.executable}\n'
                'import pathlib, sys\n'
                'arguments = sys.argv[1:]\n'
                'split = arguments.index("-o")\n'
                'output = pathlib.Path(arguments[split + 1])\n'
                'for fdid in arguments[:split]:\n'
                '    (output / f"{fdid}.blp").write_bytes(b"BLP2" + fdid.encode())\n'
            )
            extractor.chmod(0o755)
            target = root / 'player'
            artifacts = root / 'windows'
            artifacts.mkdir()
            for name in ('godot.exe', 'game_engine_godot.dll', 'game-engine-windows-x86_64.exe'):
                (artifacts / name).write_text(name)
            generated = root / 'generated'
            old_extractor = os.environ.get('CASC_LOCAL')
            os.environ['CASC_LOCAL'] = str(extractor)
            try:
                bundle.copy_data(root, target)
                sources = bundle.source_map(root, generated, artifacts)
            finally:
                if old_extractor is None:
                    os.environ.pop('CASC_LOCAL', None)
                else:
                    os.environ['CASC_LOCAL'] = old_extractor

            for fdid in expected_fdids:
                self.assertEqual(
                    (target / f'data/textures/{fdid}.blp').read_bytes(),
                    b'BLP2' + str(fdid).encode(),
                )
            self.assertEqual((target / 'data/battle-pets.json').read_text(), '[]\n')
            self.assertFalse((target / 'data/textures/0.blp').exists())
            staged_tile = generated / 'data/textures/7438338.blp'
            self.assertEqual(sources['data/textures/7438338.blp'], staged_tile)
            self.assertEqual(staged_tile.read_bytes(), b'BLP27438338')

    def test_linux_data_shares_storage_with_source_through_symlinks(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            data = root / 'data'
            for name in bundle.DATA_DIRS:
                (data / name).mkdir(parents=True)
            write_empty_ui_db2(data)
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
            write_empty_ui_db2(data)
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
            write_empty_ui_db2(data)
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

    def test_player_window_opens_maximized(self):
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory)
            bundle.write_extension_config(target, 'windows-x86_64')
            self.assertEqual((target / 'godot/override.cfg').read_text(),
                             '[display]\nwindow/size/mode=2\n')


if __name__ == '__main__':
    unittest.main()
