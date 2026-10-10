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
if target == 'test-result':
    (output / 'test.log').write_text('test result: ok. 2 passed\n')
    (output / 'status').write_text(os.environ.get('TEST_STATUS', '0'))
    sys.exit(0)
arguments = dict(value.split('=', 1) for value in json.loads(args)[1::2])
profile = 'release' if arguments.get('RELEASE') == 'true' else 'new'
binary = arguments.get('BINARY')
for name in ((binary,) if binary else ('game-server', 'game-server-admin', 'game-cli')):
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
        os.environ.pop("DEPOT_SIBLING_SHARED_PROTOCOL", None)
        os.environ.pop("DEPOT_SIBLING_GAME_ENGINE", None)
        self.engine = self.base / "game-engine"
        self.put(self.root, "data/world.db", "world rows")
        self.put(self.root, "data/game.redb", "live player storage")
        self.put(self.engine, "data/terrain/1.adt", "tile")
        self.put(self.engine, "data/terrain/1.blp", "minimap")
        self.put(self.engine, "data/los/0/32_48.los", "los tile")
        self.manifest = self.base / "test-data.txt"
        self.manifest.write_text(
            "# comment\ngame-server/world.db\ngame-engine/terrain/*.adt\ngame-engine/los/\n"
        )

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

    def test_release_snapshot_preserves_recast_build_inputs(self):
        inputs = {
            ".cargo/config.toml": '[env]\nRECAST_VENDOR = "true"\n',
            "vendor/recastnavigation-sys/CMakeLists.txt": "cmake_minimum_required(VERSION 3.5)\n",
            "vendor/recastnavigation-sys/recastnavigation/CMakeLists.txt": "project(Recast)\n",
            "vendor/recastnavigation-sys/recastnavigation/cmake/config.cmake.in": "@PACKAGE_INIT@\n",
            "vendor/recastnavigation-sys/recastnavigation/cmake/targets.cmake": "set(RECAST_FOUND TRUE)\n",
            "vendor/recastnavigation-sys/templates/header.in": "@HEADER@\n",
            "vendor/recastnavigation-sys/src/inline.cc": "void recast_inline() {}\n",
            "vendor/recastnavigation-sys/.tracked-build-input": "tracked vendor input\n",
        }
        for name, content in inputs.items():
            self.put(self.root, name, content)
        subprocess.run(["git", "-C", str(self.root), "add", ".cargo", "vendor"], check=True)
        self.put(self.root, "vendor/recastnavigation-sys/untracked.secret", "SECRET")
        module = self.load()
        with patch.object(module, "execute", self.execute):
            module.build(self.root, host="local", release=True)
        files = json.loads(self.capture.read_text())["files"]
        for name, content in inputs.items():
            self.assertEqual(files.get("game-server/" + name), content, name)
        self.assertNotIn("game-server/vendor/recastnavigation-sys/untracked.secret", files)

    def test_release_snapshot_includes_tracked_embedded_sql(self):
        # vehicle.rs embeds scripts/vehicle_models.sql with include_str!.
        self.put(self.root, "scripts/vehicle_models.sql", "SELECT 1;\n")
        subprocess.run(["git", "-C", str(self.root), "add", "scripts"], check=True)
        self.put(self.root, "scripts/untracked.sql", "SECRET")
        module = self.load()
        with patch.object(module, "execute", self.execute):
            module.build(self.root, host="local", release=True)
        files = json.loads(self.capture.read_text())["files"]
        self.assertEqual(files.get("game-server/scripts/vehicle_models.sql"), "SELECT 1;\n")
        self.assertNotIn("game-server/scripts/untracked.sql", files)

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
        for options in (
            ("--release",),
            ("--build-host", "local"),
            ("--bin", "game-cli"),
        ):
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

    def test_selected_binary_api_exports_only_requested_debug_binary(self):
        module = self.load()
        for binary in module.BINARIES:
            with self.subTest(binary=binary):
                for name in module.BINARIES:
                    self.put(self.root, f"target/debug/{name}", "old executable")
                with patch.object(module, "execute", self.execute):
                    module.build(self.root, host="local", binary=binary)
                result = json.loads(self.capture.read_text())
                self.assertIn(f"BINARY={binary}", result["args"])
                for name in module.BINARIES:
                    expected = f"new {name}" if name == binary else "old executable"
                    self.assertEqual(
                        (self.root / "target/debug" / name).read_text(), expected
                    )
                self.assertEqual(
                    (self.root / "target/debug" / binary).stat().st_mode & 0o777, 0o755
                )

    def test_selected_binary_cli_exports_only_requested_release_binary(self):
        module = self.load()
        for binary in module.BINARIES:
            with self.subTest(binary=binary):
                for name in module.BINARIES:
                    self.put(self.root, f"target/release/{name}", "old executable")
                self.assertEqual(
                    self.main(
                        module,
                        "--root",
                        str(self.root),
                        "--build-host",
                        "desktop",
                        "--release",
                        "--bin",
                        binary,
                    ),
                    0,
                )
                result = json.loads(self.capture.read_text())
                self.assertEqual(result["host"], "desktop")
                self.assertIn(f"BINARY={binary}", result["args"])
                for name in module.BINARIES:
                    expected = f"release {name}" if name == binary else "old executable"
                    self.assertEqual(
                        (self.root / "target/release" / name).read_text(), expected
                    )
                self.assertEqual(
                    (self.root / "target/release" / binary).stat().st_mode & 0o777,
                    0o755,
                )
        self.assertFalse((self.root / "target/debug").exists())

    def test_invalid_binary_rejected_before_export(self):
        module = self.load()
        with patch.object(module, "execute", self.execute):
            with self.assertRaisesRegex(ValueError, "unsupported server binary"):
                module.build(self.root, host="local", binary="unknown")
        with self.assertRaises(SystemExit) as error:
            self.main(module, "--root", str(self.root), "--bin", "unknown")
        self.assertEqual(error.exception.code, 2)
        self.assertFalse(self.capture.exists())

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

    def test_shared_protocol_sibling_override_is_snapshotted(self):
        module = self.load()
        branch = self.base / "protocol-branch"
        subprocess.run(["git", "init", "-q", str(branch)], check=True)
        self.put(branch, "Cargo.toml", "[package]\n")
        self.put(branch, "src/lib.rs", "branch protocol")
        subprocess.run(["git", "-C", str(branch), "add", "."], check=True)
        with (
            patch.dict(os.environ, {"DEPOT_SIBLING_SHARED_PROTOCOL": str(branch)}),
            patch.object(module, "execute", self.execute),
        ):
            module.build(self.root)
        files = json.loads(self.capture.read_text())["files"]
        self.assertEqual(files["shared-protocol/src/lib.rs"], "branch protocol")

    def run_test_mode(self, module, *argv):
        with patch.object(module, "TEST_DATA", self.manifest):
            return self.main(module, "--root", str(self.root), "--build-host", "local", "--test", *argv)

    def test_test_mode_syncs_listed_data_forwards_cargo_args_and_saves_log(self):
        module = self.load()
        code = self.run_test_mode(module, "-p", "server", "--", "--skip", "query 300k")
        self.assertEqual(code, 0)
        result = json.loads(self.capture.read_text())
        self.assertEqual(result["host"], "local")
        self.assertEqual(result["target"], "test-result")
        self.assertIn("TEST_ARGS=-p server -- --skip 'query 300k'", result["args"])
        self.assertIn(f"BUILD_ROOT={self.root.resolve()}", result["args"])
        mirror = self.base / "cache/game-engine/depot-build/server-test-data"
        self.assertIn(f"test-data={mirror}", result["args"])
        synced = sorted(str(p.relative_to(mirror)) for p in mirror.rglob("*") if p.is_file())
        self.assertEqual(
            synced,
            ["game-engine/los/0/32_48.los", "game-engine/terrain/1.adt", "game-server/world.db"],
        )
        self.assertEqual(
            (self.root / "target/server-test.log").read_text(), "test result: ok. 2 passed\n"
        )
        (self.engine / "data/los/0/32_48.los").unlink()
        self.put(self.engine, "data/los/0/33_48.los", "new tile")
        self.put(self.root, "data/world.db", "reimported rows")
        with patch.dict(os.environ, {"TEST_STATUS": "101"}):
            self.assertEqual(self.run_test_mode(module), 101)
        self.assertEqual((mirror / "game-server/world.db").read_text(), "reimported rows")
        self.assertFalse((mirror / "game-engine/los/0/32_48.los").exists())
        self.assertTrue((mirror / "game-engine/los/0/33_48.los").is_file())

    def test_test_mode_fails_on_missing_listed_data_before_the_host(self):
        module = self.load()
        (self.root / "data/world.db").unlink()
        self.assertEqual(self.run_test_mode(module), 1)
        self.assertFalse(self.capture.exists())

    def test_test_mode_rejects_build_options(self):
        module = self.load()
        for option in ("--release", "--bin=game-cli"):
            with self.subTest(option=option), self.assertRaises(SystemExit) as error:
                self.main(module, "--root", str(self.root), option, "--test")
            self.assertEqual(error.exception.code, 2)
        self.assertFalse(self.capture.exists())

    def test_game_engine_data_sibling_override(self):
        module = self.load()
        other = self.base / "elsewhere/game-engine"
        self.put(other, "data/terrain/2.adt", "other tile")
        self.put(other, "data/los/0/1.los", "other los")
        with patch.dict(os.environ, {"DEPOT_SIBLING_GAME_ENGINE": str(other)}):
            self.assertEqual(self.run_test_mode(module), 0)
        mirror = self.base / "cache/game-engine/depot-build/server-test-data/game-engine"
        self.assertEqual(
            sorted(str(p.relative_to(mirror)) for p in mirror.rglob("*") if p.is_file()),
            ["los/0/1.los", "terrain/2.adt"],
        )


if __name__ == "__main__":
    unittest.main()
