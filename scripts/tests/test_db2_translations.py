from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from refresh_db2_translations import regenerate_translations

GODOT = Path.home() / '.cache/game-engine/godot/4.7.2-pr123946-pr123546/godot-4.7.2-pr123946-pr123546'


class TranslationTest(unittest.TestCase):
    def test_native_imported_resources_preserve_current_csv_values(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / 'Atlas.csv'
            source.write_text('CommittedName,Width,Height\nhotkey,21,24\n')
            output = root / 'translations'
            receipt = regenerate_translations([source], output, GODOT)
            self.assertIn('Atlas.Width.translation', receipt['artifacts'])
            script = output / 'inspect.gd'
            script.write_text('extends SceneTree\nfunc _initialize():\n\tvar art = load("res://Atlas.Width.translation")\n\tif art.get_message("hotkey") != "21":\n\t\tquit(1)\n\t\treturn\n\tquit(0)\n')
            result = subprocess.run([str(GODOT), '--headless', '--path', str(output), '--script', str(script)], capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)


if __name__ == '__main__':
    unittest.main()
