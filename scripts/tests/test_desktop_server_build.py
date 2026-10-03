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
profile = 'release' if 'RELEASE=true' in json.loads(args) else 'new'
for name in ('game-server', 'game-server-admin', 'game-cli'):
    content = (profile + ' ' + name).encode()
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
        self.setting = self.base / "config/game-engine/build-host"
        if not self.setting.exists():
            self.put(self.base, "config/game-engine/build-host", "desktop\n")
        setting = patch.object(module.depot, "build_host_setting", lambda: self.setting)
        setting.start()
        self.addCleanup(setting.stop)
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

    def main(self, module, *args):
        with (
            patch.object(sys, "argv", ["desktop-server-build.py", *args]),
            patch.object(module, "execute", self.execute),
        ):
            return module.main()

    def test_local_debug_uses_absolute_checkout_build_contract(self):
        module = self.load()
        with patch.object(module, "execute", self.execute):
            module.build(self.root / "crates/..", host="local")
        result = json.loads(self.capture.read_text())
        self.assertEqual(result["host"], "local")
        self.assertIn(f"BUILD_ROOT={self.root.resolve()}", result["args"])
        self.assertIn(f"BUILD_PARENT={self.root.parent.resolve()}", result["args"])
        self.assertIn("RELEASE=false", result["args"])
        self.assertIn("SERVER_CACHE=" + result["key"], result["args"])
        for name in module.BINARIES:
            path = self.root / "target/debug" / name
            self.assertEqual(path.read_bytes(), ("new " + name).encode())
            self.assertEqual(path.stat().st_mode & 0o777, 0o755)
        self.assertFalse((self.root / "target/release").exists())

    def test_release_cli_exports_all_binaries_without_overwriting_debug(self):
        module = self.load()
        for name in module.BINARIES:
            self.put(self.root, f"target/debug/{name}", "old debug")
        self.assertEqual(
            self.main(
                module, "--root", str(self.root), "--build-host", "local", "--release"
            ),
            0,
        )
        result = json.loads(self.capture.read_text())
        self.assertEqual(result["host"], "local")
        self.assertIn("RELEASE=true", result["args"])
        for name in module.BINARIES:
            path = self.root / "target/release" / name
            self.assertEqual(path.read_bytes(), ("release " + name).encode())
            self.assertEqual(path.stat().st_mode & 0o777, 0o755)
            self.assertEqual(
                (self.root / "target/debug" / name).read_text(), "old debug"
            )

    def test_saved_local_host_applies_to_legacy_build_api(self):
        module = self.load()
        self.assertEqual(self.main(module, "--save-build-host", "local"), 0)
        self.assertEqual(self.setting.read_text(), "local\n")
        self.assertFalse(self.capture.exists())
        self.assertFalse((self.root / "target/debug").exists())
        with patch.object(module, "execute", self.execute):
            module.build(self.root)
        self.assertEqual(json.loads(self.capture.read_text())["host"], "local")
        self.assertEqual(self.main(module, "--save-build-host", "desktop"), 0)
        self.assertEqual(self.setting.read_text(), "desktop\n")
        with patch.object(module, "execute", self.execute):
            module.build(self.root)
        self.assertEqual(json.loads(self.capture.read_text())["host"], "desktop")

    def test_missing_or_invalid_setting_fails_without_transport(self):
        module = self.load()
        self.setting.unlink()
        with patch.object(module, "execute", self.execute):
            with self.assertRaisesRegex(ValueError, "choose --build-host"):
                module.build(self.root)
            self.assertFalse(self.capture.exists())
            self.setting.write_text("unknown\n")
            with self.assertRaisesRegex(ValueError, "invalid build-host"):
                module.build(self.root)
            self.assertFalse(self.capture.exists())
            module.build(self.root, host="desktop")
        self.assertEqual(json.loads(self.capture.read_text())["host"], "desktop")
        self.assertEqual(self.setting.read_text(), "unknown\n")

    def test_save_host_rejects_build_options_without_saving_or_building(self):
        module = self.load()
        for options in (("--release",), ("--build-host", "local")):
            with self.subTest(options=options):
                with self.assertRaises(SystemExit) as error:
                    self.main(module, "--save-build-host", "local", *options)
                self.assertEqual(error.exception.code, 2)
                self.assertEqual(self.setting.read_text(), "desktop\n")
                self.assertFalse(self.capture.exists())

    def test_release_failure_preserves_old_artifacts_without_host_fallback(self):
        module = self.load()
        for name in module.BINARIES:
            self.put(self.root, f"target/release/{name}", "old release")
        for failure in ("transport", "game-server"):
            with (
                self.subTest(failure=failure),
                patch.dict(os.environ, {"FAIL_EXPORT": failure}),
            ):
                self.assertEqual(
                    self.main(
                        module,
                        "--root",
                        str(self.root),
                        "--build-host",
                        "local",
                        "--release",
                    ),
                    1,
                )
                self.assertEqual(json.loads(self.capture.read_text())["host"], "local")
                for name in module.BINARIES:
                    path = self.root / "target/release" / name
                    self.assertEqual(path.read_text(), "old release")
                    self.assertEqual(list(path.parent.glob(f".{name}-*")), [])
        self.assertFalse((self.root / "target/debug").exists())

    def test_release_target_symlink_rejected_before_export(self):
        module = self.load()
        outside = self.base / "outside"
        outside.mkdir()
        (self.root / "target/release").symlink_to(outside, target_is_directory=True)
        with patch.object(module, "execute", self.execute):
            with self.assertRaisesRegex(ValueError, "target symlink"):
                module.build(self.root, host="local", release=True)
        self.assertFalse(self.capture.exists())
        self.assertEqual(list(outside.iterdir()), [])

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
