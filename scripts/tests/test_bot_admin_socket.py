"""Admin subprocess receives the private Stockade server port's socket."""
import os
from pathlib import Path
import sys
import tempfile
from types import SimpleNamespace
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from run_stockade_bot import StockadeRun


class StockadeAdminSocketTests(unittest.TestCase):
    @unittest.skipUnless(os.environ.get("BOT_ADMIN_RED"), "behavioral RED: private server port not wired")
    def test_admin_subprocess_socket_follows_explicit_server_port(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            program = root / "target/debug/game-server-admin"
            program.parent.mkdir(parents=True)
            program.write_text("#!/usr/bin/env python3\nimport os\nprint(os.environ['GAME_SERVER_ADMIN_SOCKET'])\n")
            program.chmod(0o755)
            controller = StockadeRun.__new__(StockadeRun)
            controller.args = SimpleNamespace(server=root, server_port=5981)
            controller.output = root
            controller.sequence = 0
            self.assertEqual(controller.admin("socket-proof").strip(), "/tmp/game-server-admin-5981.sock")


if __name__ == "__main__":
    unittest.main()
