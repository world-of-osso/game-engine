import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
import sys
sys.path.insert(0, str(Path(__file__).parents[1]))

SCRIPT = Path(__file__).parents[1] / 'migrate_model_asset_index.py'


class ModelAssetMigrationTests(unittest.TestCase):
    def test_old_build_assets_and_aliases_follow_new_identity_without_duplicates(self):
        spec = importlib.util.spec_from_file_location('migration', SCRIPT)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            data, output = root / 'data', root / 'staged'
            (data / 'cache').mkdir(parents=True)
            old = {'product': 'wow', 'build_key': '1' * 32, 'build': '12.1.0.69933'}
            new = {'product': 'wow', 'build_key': '2' * 32, 'build': '12.1.5.70391'}
            rows = []
            roots, extracted = {}, {}
            for fdid, original, current in [(126862, b'BLP2same', b'BLP2same'), (126863, b'BLP2old', b'BLP2new')]:
                path = f'products/wow/{old["build_key"]}/textures/{fdid}.blp'
                target = data / path
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(original)
                row = {**old, 'fdid': fdid, 'kind': 'blp', 'path': path, 'bytes': len(original), 'sha256': hashlib.sha256(original).hexdigest(), 'content_key': hashlib.md5(original).hexdigest()}
                rows.append(row)
                roots[fdid] = hashlib.md5(current).digest()
                if original != current:
                    raw = root / f'{fdid}.dat'
                    raw.write_bytes(current)
                    extracted[fdid] = raw
            alias = {**rows[1], 'path': f'products/wow/{old["build_key"]}/textures/alias.blp'}
            (data / alias['path']).write_bytes(b'BLP2old')
            for name, row in zip(['model-asset-index.json', 'model-asset-index-bg.json'], rows):
                (data / 'cache' / name).write_text(json.dumps({'version': 1, 'assets': [row], 'aliases': [alias] if row['fdid'] == 126863 else [], 'chains': [old]}))
            manifest, report = module.prepare_migration(data, output, new, roots, extracted)
            from apply_db2_install import apply
            apply_manifest = output / 'apply.json'
            apply_manifest.write_text(json.dumps(manifest))
            apply(apply_manifest, root / 'backup')
            published = [row for p in (data / 'cache').glob('model-asset-index*.json') for row in json.loads(p.read_text())['assets']]
            self.assertEqual(len(published), 2)
            self.assertEqual({row['build_key'] for row in published}, {new['build_key']})
            self.assertEqual(report['unchanged_bytes'], 1)
            self.assertEqual(report['changed_bytes'], 1)
            by_id = {row['fdid']: row for row in published}
            self.assertEqual((data / by_id[126862]['path']).read_bytes(), b'BLP2same')
            self.assertEqual((data / by_id[126863]['path']).read_bytes(), b'BLP2new')
            migrated_alias = json.loads((data / 'cache/model-asset-index-bg.json').read_text())['aliases'][0]
            self.assertEqual(migrated_alias['build_key'], new['build_key'])
            self.assertEqual((data / migrated_alias['path']).read_bytes(), b'BLP2new')
            self.assertEqual((data / by_id[126862]['path']).stat().st_ino, (data / rows[0]['path']).stat().st_ino)
            self.assertTrue((root / 'backup/fingerprint-diff.json').exists())

    def test_alias_without_content_key_is_reextracted_and_keeps_owner_metadata(self):
        spec = importlib.util.spec_from_file_location('migration', SCRIPT)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            data, output = root / 'data', root / 'staged'
            (data / 'cache').mkdir(parents=True)
            old = {'product': 'wow', 'build_key': '1' * 32, 'build': '12.1.0.69933'}
            new = {'product': 'wow', 'build_key': '2' * 32, 'build': '12.1.5.70391'}
            original, current = b'SKIN-old', b'SKIN-new'
            path = f'products/wow/{old["build_key"]}/models/11741200.skin'
            (data / path).parent.mkdir(parents=True, exist_ok=True)
            (data / path).write_bytes(original)
            # NPC-appearance alias rows carry sha256 but no CASC content key.
            alias = {**old, 'fdid': 469942, 'kind': 'skin', 'path': path, 'owner_model_fdid': 117412,
                     'reference': 'SFID[0]', 'bytes': len(original), 'sha256': hashlib.sha256(original).hexdigest()}
            (data / 'cache' / 'model-asset-index.json').write_text(
                json.dumps({'version': 1, 'assets': [], 'aliases': [], 'chains': []}))
            (data / 'cache' / 'model-asset-index-npc.json').write_text(
                json.dumps({'version': 1, 'assets': [], 'aliases': [alias], 'chains': []}))
            raw = root / '469942.dat'
            raw.write_bytes(current)
            manifest, _ = module.prepare_migration(data, output, new, {469942: hashlib.md5(current).digest()}, {469942: raw})
            from apply_db2_install import apply
            apply_manifest = output / 'apply.json'
            apply_manifest.write_text(json.dumps(manifest))
            apply(apply_manifest, root / 'backup')
            migrated = json.loads((data / 'cache/model-asset-index-npc.json').read_text())['aliases'][0]
            self.assertEqual(migrated['build_key'], new['build_key'])
            self.assertEqual(migrated['owner_model_fdid'], 117412)
            self.assertEqual(migrated['reference'], 'SFID[0]')
            self.assertEqual(migrated['content_key'], hashlib.md5(current).hexdigest())
            self.assertEqual((data / migrated['path']).read_bytes(), current)

    def test_installed_migration_treats_missing_content_key_as_needing_extraction(self):
        spec = importlib.util.spec_from_file_location('migration', SCRIPT)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        row = {'product': 'wow', 'fdid': 469942}
        self.assertTrue(module.needs_extraction(row, {469942: b'\x01' * 16}))
        self.assertFalse(module.needs_extraction({**row, 'content_key': '01' * 16}, {469942: b'\x01' * 16}))
        self.assertFalse(module.needs_extraction(row, {}))


    def test_unreadable_current_bytes_are_reported_and_dropped_not_published(self):
        spec = importlib.util.spec_from_file_location('migration', SCRIPT)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            data, output = root / 'data', root / 'staged'
            (data / 'cache').mkdir(parents=True)
            old = {'product': 'wow', 'build_key': '1' * 32, 'build': '12.1.0.69933'}
            new = {'product': 'wow', 'build_key': '2' * 32, 'build': '12.1.5.70391'}
            rows, roots, extracted = [], {}, {}
            for fdid, kind, original, current in [(3269019, 'skin', b'SKIN-old', b'SKIN-real-current'), (126863, 'blp', b'BLP2old', b'BLP2new')]:
                path = f'products/wow/{old["build_key"]}/{"models" if kind == "skin" else "textures"}/{fdid}.{kind}'
                (data / path).parent.mkdir(parents=True, exist_ok=True)
                (data / path).write_bytes(original)
                rows.append({**old, 'fdid': fdid, 'kind': kind, 'path': path, 'bytes': len(original),
                             'sha256': hashlib.sha256(original).hexdigest(), 'content_key': hashlib.md5(original).hexdigest()})
                roots[fdid] = hashlib.md5(current).digest()
                raw = root / f'{fdid}.dat'
                # Encrypted content without a key extracts as zeros, which cannot match the root content key.
                raw.write_bytes(b'\0' * len(current) if fdid == 3269019 else current)
                extracted[fdid] = raw
            (data / 'cache' / 'model-asset-index.json').write_text(
                json.dumps({'version': 1, 'assets': rows, 'aliases': [], 'chains': []}))
            manifest, report = module.prepare_migration(data, output, new, roots, extracted)
            from apply_db2_install import apply
            apply_manifest = output / 'apply.json'
            apply_manifest.write_text(json.dumps(manifest))
            apply(apply_manifest, root / 'backup')
            published = json.loads((data / 'cache/model-asset-index.json').read_text())['assets']
            self.assertEqual([row['fdid'] for row in published], [126863])
            self.assertEqual([row['fdid'] for row in report['unreadable']], [3269019])
            self.assertFalse((data / f'products/wow/{new["build_key"]}/models/3269019.skin').exists())


if __name__ == '__main__':
    unittest.main()
