"""Source snapshot and executable export proof without Docker or SSH."""

import gzip
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

SCRIPTS = Path(__file__).resolve().parents[1]
FAKE = r"""#!/usr/bin/env python3
import gzip, json, os
from pathlib import Path
import sys
context, output, key, target, args, host = sys.argv[1:]
context, output = Path(context), Path(output)
files = {str(p.relative_to(context)): p.read_text() for p in context.rglob('*') if p.is_file()}
Path(os.environ['CAPTURE']).write_text(json.dumps(dict(files=files, context=str(context), key=key, target=target, args=json.loads(args), host=host)))
if os.environ.get('FAIL_EXPORT') == 'transport':
    sys.exit(7)
for name in ('game-server', 'game-server-admin', 'game-cli'):
    content = ('new ' + name).encode()
    (output / (name + '.gz')).write_bytes(b'broken' if os.environ.get('FAIL_EXPORT') == name else gzip.compress(content))
"""


class DesktopServerBuildTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name)
        self.root = self.base / "game-server"
        self.shared = self.base / "shared-protocol"
        for repo in (self.root, self.shared):
            repo.mkdir()
            subprocess.run(["git", "init", "-q", str(repo)], check=True)
        self.put(self.root, "Cargo.toml", "[workspace]\n")
        self.put(self.root, "Cargo.lock", "locked")
        self.put(self.root, "crates/server/src/main.rs", "old source")
        self.put(self.root, "crates/server/src/ground_gaps.tsv", "0\tmissing_tile\n")
        self.put(self.root, "vendor/bevy_replicon/src/lib.rs", "replicon patch")
        self.put(
            self.root, "vendor/lightyear_replication/src/lib.rs", "replication patch"
        )
        self.put(self.shared, "Cargo.toml", "[package]\n")
        self.put(self.shared, "src/lib.rs", "shared source")
        self.put(self.shared, "vendor/lightyear_netcode/src/lib.rs", "netcode patch")
        for repo in (self.root, self.shared):
            subprocess.run(["git", "-C", str(repo), "add", "."], check=True)
        self.put(self.root, "crates/server/src/main.rs", "working source")
        self.put(self.root, "crates/server/src/new.rs", "untracked source")
        self.put(self.root, "crates/server/src/untracked.tsv", "not a compile input")
        for name in ("data/secret.rs", "target/leak.rs", ".env", "token.txt"):
            self.put(self.root, name, "SECRET")
        self.fake = self.base / "fake-transport"
        self.fake.write_text(FAKE)
        self.fake.chmod(0o755)
        self.capture = self.base / "capture.json"
        self.environment = patch.dict(
            os.environ,
            {"XDG_CACHE_HOME": str(self.base / "cache"), "CAPTURE": str(self.capture)},
        )
        self.environment.start()
        self.addCleanup(self.environment.stop)

    def put(self, root, name, content):
        path = root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content)

    def load(self):
        script = SCRIPTS / "desktop-server-build.py"
        self.assertTrue(script.is_file(), "desktop server build helper missing")
        sys.path.insert(0, str(SCRIPTS))
        self.addCleanup(lambda: sys.path.remove(str(SCRIPTS)))
        spec = importlib.util.spec_from_file_location("desktop_server_build", script)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return module

    def execute(self, context, output, key, target, args, host):
        subprocess.run(
            [
                str(self.fake),
                str(context),
                str(output),
                key,
                target,
                json.dumps(args),
                host,
            ],
            check=True,
        )

    def test_working_snapshot_and_three_executable_exports(self):
        module = self.load()
        with patch.object(module, "execute", self.execute):
            module.build(self.root)
        result = json.loads(self.capture.read_text())
        files = result["files"]
        self.assertEqual(
            files["game-server/crates/server/src/main.rs"], "working source"
        )
        self.assertEqual(
            files["game-server/crates/server/src/new.rs"], "untracked source"
        )
        self.assertEqual(
            files["game-server/crates/server/src/ground_gaps.tsv"], "0\tmissing_tile\n"
        )
        self.assertEqual(
            files["game-server/vendor/bevy_replicon/src/lib.rs"], "replicon patch"
        )
        self.assertEqual(
            files["game-server/vendor/lightyear_replication/src/lib.rs"],
            "replication patch",
        )
        self.assertEqual(
            files["shared-protocol/vendor/lightyear_netcode/src/lib.rs"],
            "netcode patch",
        )
        self.assertNotIn("game-server/crates/server/src/untracked.tsv", files)
        self.assertNotIn("SECRET", files.values())
        self.assertEqual(result["host"], "desktop")
        self.assertEqual(result["target"], "artifact")
        self.assertIn("TARGET_CACHE=server-target-" + result["key"], result["args"])
        for name in module.BINARIES:
            path = self.root / "target/debug" / name
            self.assertEqual(path.read_bytes(), ("new " + name).encode())
            self.assertEqual(path.stat().st_mode & 0o777, 0o755)
        first_context = result["context"]
        self.put(self.root, "crates/server/src/main.rs", "changed again")
        (self.root / "crates/server/src/new.rs").unlink()
        with patch.object(module, "execute", self.execute):
            module.build(self.root)
        result = json.loads(self.capture.read_text())
        self.assertEqual(result["context"], first_context)
        self.assertEqual(
            result["files"]["game-server/crates/server/src/main.rs"], "changed again"
        )
        self.assertNotIn("game-server/crates/server/src/new.rs", result["files"])

    def test_failure_preserves_old_executable(self):
        module = self.load()
        old = self.root / "target/debug/game-server"
        old.parent.mkdir(parents=True)
        old.write_bytes(b"old executable")
        old.chmod(0o755)
        for failure in ("transport", "game-server"):
            with (
                self.subTest(failure=failure),
                patch.dict(os.environ, {"FAIL_EXPORT": failure}),
                patch.object(module, "execute", self.execute),
            ):
                with self.assertRaises(
                    (subprocess.CalledProcessError, gzip.BadGzipFile)
                ):
                    module.build(self.root)
                self.assertEqual(old.read_bytes(), b"old executable")
                self.assertEqual(old.stat().st_mode & 0o777, 0o755)
                self.assertEqual(list(old.parent.glob(".game-server-*")), [])

    def test_checkout_keys_are_distinct(self):
        module = self.load()
        with patch.object(module, "execute", self.execute):
            module.build(self.root)
            first = json.loads(self.capture.read_text())["key"]
            other = self.base / "other" / "game-server"
            import shutil

            shutil.copytree(self.root, other)
            shutil.copytree(self.shared, other.parent / "shared-protocol")
            module.build(other)
            self.assertNotEqual(json.loads(self.capture.read_text())["key"], first)


if __name__ == "__main__":
    unittest.main()
