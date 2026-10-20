"""Out-of-checkout server invocations stage tracked catalogs before the child starts."""
import importlib
import json
import os
from pathlib import Path
import sys
import tempfile
import threading
import unittest
from unittest import mock

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
native = importlib.import_module("native_build_hosts")
CATALOGS = ("trading-post-monthly-catalog.json", "quest-reward-bonus.json")


class ServerRuntimeCatalogTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.base = Path(self.directory.name)
        self.project = self.base / "source"
        self.runtime = self.base / "private-runtime"
        self.target = self.base / "target"
        (self.project / "content").mkdir(parents=True)
        (self.runtime / "data").mkdir(parents=True)
        (self.target / "debug").mkdir(parents=True)
        for name in CATALOGS:
            (self.project / "content" / name).write_text(json.dumps({"source": name}))
        self.observed = self.base / "observed.json"
        self.server = self.target / "debug/game-server"
        self.server.write_text("""#!/usr/bin/python3
from pathlib import Path
import json,os
catalogs=['trading-post-monthly-catalog.json','quest-reward-bonus.json']
result={name:Path('data',name).read_text() for name in catalogs}
Path(os.environ['OBSERVED']).write_text(json.dumps(result))
""")
        self.server.chmod(0o755)

    def invoke(self, cwd):
        with mock.patch.object(native, "read_sysroot", return_value=(0, Path("/unused-sysroot"))):
            return native.native_runtime(
                self.project, self.target, [], "game-server", False,
                {**os.environ, "OBSERVED": str(self.observed)}, [],
                (threading.Event(), [0]), runtime_cwd=cwd,
            )

    def test_private_runtime_receives_tracked_catalog_bytes_before_child_start(self):
        status = self.invoke(self.runtime)
        self.assertEqual(status, 0)
        expected = {name: (self.project / "content" / name).read_text() for name in CATALOGS}
        self.assertEqual(json.loads(self.observed.read_text()), expected)

    def test_missing_tracked_bonus_refuses_launch_even_with_stale_runtime_file(self):
        for name in CATALOGS:
            (self.runtime / "data" / name).write_text("stale runtime value")
        (self.project / "content/quest-reward-bonus.json").unlink()
        with self.assertRaises(FileNotFoundError) as failure:
            self.invoke(self.runtime)
        self.assertIn("quest-reward-bonus.json", str(failure.exception))
        self.assertFalse(self.observed.exists(), "child must not run with stale catalog")

    def test_in_checkout_run_reads_existing_data_without_rewriting_caller_files(self):
        data = self.project / "data"
        data.mkdir()
        for name in CATALOGS:
            (data / name).write_bytes((self.project / "content" / name).read_bytes())
            os.utime(data / name, (1_000_000, 1_000_000))
        before = {name: (data / name).stat().st_mtime_ns for name in CATALOGS}
        self.assertEqual(self.invoke(self.project), 0)
        self.assertEqual({name: (data / name).stat().st_mtime_ns for name in CATALOGS}, before)


if __name__ == "__main__":
    unittest.main()
