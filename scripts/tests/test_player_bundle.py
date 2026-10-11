import importlib.util
import tempfile
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location('player_bundle', Path(__file__).parents[1] / 'player_bundle.py')
bundle = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bundle)


class PlayerBundleTest(unittest.TestCase):
    def test_allowlist_excludes_credentials_and_extraction_state(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            data = root / 'data'
            for name in bundle.DATA_DIRS:
                (data / name).mkdir(parents=True)
            for name in ('cache/catalog.sqlite', 'cache/casc/root.bin', 'cache/pre-retail-old/a',
                         'textures/12.blp', 'textures/13.blp.missing', 'cache/a.lock',
                         'auth_token.realm', 'diagnostics/run.log', 'Map.csv', 'local-listfile-cache.sqlite',
                         'forever-1.60.1.70205/models/a.m2', 'gametables/classes.txt'):
                path = data / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(name)
            expected = {'cache/catalog.sqlite', 'textures/12.blp', 'Map.csv',
                        'local-listfile-cache.sqlite', 'forever-1.60.1.70205/models/a.m2',
                        'gametables/classes.txt'}
            selected = list(bundle.data_files(root))
            self.assertEqual({str(p) for p in selected}, expected)
            target = root / 'player'
            bundle.copy_data(root, target)
            self.assertEqual({str(p.relative_to(target / 'data')) for p in (target / 'data').rglob('*') if p.is_file()}, expected)
            for relative in selected:
                self.assertEqual((target / 'data' / relative).read_bytes(), (data / relative).read_bytes())

    def test_platform_extension_contract(self):
        for platform, library in [('linux-x86_64', 'linux.x86_64 = "res://../lib/libgame_engine_godot.so"'),
                                  ('windows-x86_64', 'windows.x86_64 = "res://../lib/game_engine_godot.dll"')]:
            with tempfile.TemporaryDirectory() as directory:
                target = Path(directory)
                bundle.write_extension_config(target, platform)
                self.assertIn(library, (target / 'godot/game_engine.gdextension').read_text())


if __name__ == '__main__':
    unittest.main()
