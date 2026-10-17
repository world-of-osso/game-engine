"""Live escort adapter uses the active export, not an unfrozen historical path."""
import hashlib
import json
from pathlib import Path
import sqlite3
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / 'scripts/prepare_escort_appearance.py'
DATA = Path('/home/osso/Projects/world-of-osso/game-engine/data')
WORLD = Path('/home/osso/Projects/world-of-osso/game-server/data/world.db')


class EscortActiveSourceTests(unittest.TestCase):
    def test_projection_reads_only_active_csvs_and_preserves_all_selected_display_coverage(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'data/db2').mkdir(parents=True)
            (root / 'scripts').symlink_to(ROOT / 'scripts', target_is_directory=True)
            (root / 'data/db2/active').symlink_to(DATA / 'db2/active', target_is_directory=True)
            (root / 'data/cache').symlink_to(DATA / 'cache', target_is_directory=True)
            (root / 'data/community-listfile.csv').symlink_to(DATA / 'community-listfile.csv')
            output = root / 'escort.sqlite'
            with sqlite3.connect(WORLD.resolve().as_uri() + '?mode=ro', uri=True) as connection:
                expected = {row[0] for row in connection.execute('SELECT CreatureDisplayID FROM content_creature_template_model WHERE CreatureID IN (4484,3663,3879,3893,3898,3899,3900)')}
            self.assertTrue(expected)
            result = subprocess.run([sys.executable, str(SCRIPT), '--root', str(root),
                                     '--source-world', str(WORLD), '--output', str(output)],
                                    capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            manifest = json.loads(output.with_suffix('.json').read_text())
            self.assertEqual(set(manifest['selected_displays']), expected)
            self.assertEqual(manifest['csv_root'], str((root / 'data/db2/active').resolve()))
            for name, digest in manifest['sources_sha256'].items():
                self.assertEqual(digest, hashlib.sha256((root / 'data/db2/active' / name).read_bytes()).hexdigest())
            with sqlite3.connect(output) as connection:
                covered = {row[0] for row in connection.execute('SELECT display_id FROM display_coverage')}
                self.assertTrue(expected.issubset(covered))
                self.assertEqual(connection.execute('PRAGMA quick_check').fetchall(), [('ok',)])


if __name__ == '__main__':
    unittest.main()
