"""Desktop smoke prerequisite checks without starting a real realm or using SSH."""
import importlib.util
from pathlib import Path
import tempfile
import unittest

SCRIPT = Path(__file__).with_name("desktop_server_smoke.py")
spec = importlib.util.spec_from_file_location("desktop_server_smoke", SCRIPT)
smoke = importlib.util.module_from_spec(spec)
spec.loader.exec_module(smoke)


class DesktopServerSmokeDataTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        (self.root / "bin").mkdir()
        (self.root / "data/economy").mkdir(parents=True)
        (self.root / "data/world.db").write_text("fixture world")
        (self.root / "data/economy/config.json").write_text("{}")
        (self.root / "data/trading-post-monthly-catalog.json").write_text('{"month":46}')
        self.program("game-server", """from pathlib import Path
import os,time
root=Path.cwd()
(root/'started').write_text('child started')
Path(os.environ['GAME_SERVER_ADMIN_SOCKET']).touch()
while True: time.sleep(.05)
""")
        self.program("game-server-admin", "import sys\nprint('pong' if sys.argv[1]=='ping' else 'created')\n")
        self.program("game-cli", "print('[login] Characters: 0')\n")

    def program(self, name, body):
        path = self.root / "bin" / name
        path.write_text("#!/usr/bin/python3\n" + body)
        path.chmod(0o755)

    def test_missing_bonus_is_named_before_any_server_process_starts(self):
        with self.assertRaises(FileNotFoundError) as failure:
            smoke.smoke(self.root)
        self.assertIn("quest-reward-bonus.json", str(failure.exception))
        self.assertFalse((self.root / "started").exists())

    def test_complete_staged_catalogs_are_preserved_through_process_lifecycle(self):
        catalog = self.root / "data/quest-reward-bonus.json"
        catalog.write_text('[{"quest_id":62,"bonus_money":120}]')
        before = catalog.read_bytes()
        smoke.smoke(self.root)
        self.assertTrue((self.root / "started").exists())
        self.assertEqual(catalog.read_bytes(), before)


if __name__ == "__main__":
    unittest.main()
