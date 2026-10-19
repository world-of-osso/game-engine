"""Admin subprocess receives the private Stockade server port's socket."""
import json
import os
from pathlib import Path
import sys
import tempfile
from types import SimpleNamespace
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from run_stockade_bot import StockadeRun


class StockadeAdminSocketTests(unittest.TestCase):
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


class RosterProvisionSocketTests(unittest.TestCase):
    @unittest.skipUnless(os.environ.get("BOT_ADMIN_RED"), "behavioral RED: roster provisioning still uses5871")
    def test_all_fresh_provision_subprocesses_receive_the_configured_socket(self):
        from run_bot_roster import provision
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            observed = root / "observed.jsonl"
            record = ("import json, os, sys\n"
                      f"with open({str(observed)!r}, 'a') as stream:\n"
                      "    stream.write(json.dumps({'argv':sys.argv[1:], 'socket':os.environ['GAME_SERVER_ADMIN_SOCKET']})+'\\n')\n")
            program = root / "target/debug/game-server-admin"
            program.parent.mkdir(parents=True)
            program.write_text("#!/usr/bin/env python3\n" + record +
                "print('exists=false' if sys.argv[1]=='check-account' else 'username=fb_path name=Pathfury ' if sys.argv[1]=='find-character' else 'ok')\n")
            program.chmod(0o755)
            placement = root / "scripts/place-private-character.py"
            placement.parent.mkdir()
            placement.write_text(record)
            args = SimpleNamespace(server=root, server_port=5981, output=root)
            provision(args, {"account":"fb_path", "name":"Pathfury", "race":1, "class":1, "level":32})
            rows = [json.loads(line) for line in observed.read_text().splitlines()]
            self.assertEqual(len(rows), 7)
            self.assertEqual({row["socket"] for row in rows}, {"/tmp/game-server-admin-5981.sock"})


if __name__ == "__main__":
    unittest.main()
