import importlib.util
import tempfile
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location('import_project', Path(__file__).parents[1] / 'import_project.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class ImportProjectTest(unittest.TestCase):
    def test_import_can_read_data_but_cannot_modify_source(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / 'root'
            data = root / 'data'
            data.mkdir(parents=True)
            source = data / 'catalog'
            source.write_text('original')
            generated = root / 'generated'
            (generated / 'godot').mkdir(parents=True)
            executable = generated / 'godot-fixture'
            executable.write_text('''#!/usr/bin/python3
import pathlib, sys
project = pathlib.Path(sys.argv[sys.argv.index('--path') + 1])
source = project.parent / 'data/catalog'
assert source.read_text() == 'original'
try:
    source.write_text('modified')
except PermissionError:
    (project / 'imported').write_text('ok')
else:
    raise SystemExit('source was writable')
''')
            executable.chmod(0o755)
            module.import_project(root, generated, executable)
            self.assertEqual(source.read_text(), 'original')
            self.assertEqual((generated / 'godot/imported').read_text(), 'ok')
            self.assertFalse((generated / 'data').exists())


if __name__ == '__main__':
    unittest.main()
