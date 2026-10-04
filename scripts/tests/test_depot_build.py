"""Behavioral tests for source-only Depot builds without network or Cargo."""

import json
import os
import subprocess
import tempfile
import time
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "depot-build.py"
REFRESH = SCRIPT.parent / "depot" / "refresh-source-mtimes.py"
FIXTURES = (
    "native_input_fixture",
    "native_npc_visual_fixture",
    "native_reconnect_fixture",
    "native_transfer_fixture",
)
SIBLINGS = (
    "asset-resolver",
    "ui-toolkit",
    "ui-toolkit-macros",
    "shared-protocol",
)
FAKE_DEPOT = """#!/usr/bin/env python3
import gzip, json, os, pathlib, sys, time
args = sys.argv[1:]
assert args[:2] == ['buildx', 'build'], args
project = args[args.index('--builder') + 1]
output = pathlib.Path(args[args.index('--output') + 1].split('dest=', 1)[1])
context = pathlib.Path(args[-1])
files = {str(p.relative_to(context)): p.read_bytes().decode('latin1') for p in context.rglob('*') if p.is_file()}
mtimes = {str(p.relative_to(context)): p.stat().st_mtime_ns for p in context.rglob('*') if p.is_file()}
with open(os.environ['DEPOT_RECORD'], 'a') as record:
    record.write(json.dumps({'project': project, 'files': files, 'mtimes': mtimes, 'context': str(context), 'args': args,
                             'xdg_config_home': os.environ.get('XDG_CONFIG_HOME'), 'has_token': 'DEPOT_TOKEN' in os.environ}) + '\\n')
if os.environ.get('DEPOT_DELAY'):
    time.sleep(float(os.environ['DEPOT_DELAY']))
if os.environ.get('DEPOT_FAIL'):
    sys.exit(7)
output.mkdir(parents=True, exist_ok=True)
if args[args.index('--target') + 1] == 'test-result':
    assets = context / 'test-assets'
    staged = {str(p.relative_to(assets)): p.read_text() for p in assets.rglob('*') if p.is_file()}
    with open(os.environ['DEPOT_RECORD'], 'a') as record:
        record.write(json.dumps({'assets': staged, 'asset_dir': str(assets)}) + '\\n')
    if not os.environ.get('DEPOT_NO_LOG'):
        (output / 'test.log').write_text('test result: ' + os.environ.get('DEPOT_TEST_LOG', 'ok') + '\\n')
        (output / 'status').write_text(os.environ.get('DEPOT_TEST_STATUS', '0') + '\\n')
    sys.exit(0)
artifact = output / 'libgame_engine_godot.so.gz'
artifact.write_bytes(b'bad gzip' if os.environ.get('DEPOT_CORRUPT') else gzip.compress(os.environ.get('DEPOT_ARTIFACT', 'new binary').encode()))
build_args = [args[index + 1] for index, arg in enumerate(args) if arg == '--build-arg']
options = dict(option.split('=', 1) for option in build_args)
assert len(options) == len(build_args), build_args
if options.get('CLI') == '1':
    (output / 'game-engine-cli.gz').write_bytes(gzip.compress(b'cli binary'))
fixture = options.get('FIXTURE')
if fixture and not os.environ.get('DEPOT_MISSING_FIXTURE'):
    (output / (fixture + '.gz')).write_bytes(b'bad gzip' if os.environ.get('DEPOT_CORRUPT_FIXTURE') else gzip.compress(os.environ.get('DEPOT_FIXTURE', 'fixture binary').encode()))
"""


class DepotBuildTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.base = Path(self.tmp.name)
        self.root = self.base / "game-engine-godot-conversion"
        self.root.mkdir()
        (self.root / "godot").mkdir()
        self._put(self.root, "godot/Cargo.toml", "[workspace]\nmembers=[]\n")
        self._put(self.root, "godot/Cargo.lock", "lock")
        for member in ("cli", "core", "network", "rust", "session", "ui-model"):
            self._put(
                self.root,
                f"godot/{member}/Cargo.toml",
                "[package]\nname='fixture'\nversion='0.1.0'\n",
            )
        self._put(self.root, "godot/rust/src/lib.rs", "original")
        for name in FIXTURES:
            self._put(self.root, f"godot/network/examples/{name}.rs", "fn main() {}")
        self._put(self.root, "godot/network/examples/fixture_support/mod.rs", "shared")
        self._put(self.root, "godot/core/src/asset/mod.rs", "asset")
        self._put(
            self.root,
            "godot/rust/src/rendering/ui/nameplate_skins/health-fill.png",
            "png",
        )
        for crate in ("taffy", "ktx2-rw"):
            self._put(
                self.root,
                f"vendor/{crate}/Cargo.toml",
                "[package]\nname='fixture'\nversion='0.1.0'\n",
            )
            self._put(self.root, f"vendor/{crate}/src/lib.rs", "vendored patch")
        self._put(self.root, "vendor/taffy/README.md", "required compile include")
        self._put(self.root, "src/legacy.rs", "retired root crate")
        self._put(self.root, "data/private.rs", "private")
        self._put(self.root, "data/models/boar.m2", "boar model")
        self._put(self.root, "data/Light.csv", "light rows")
        self._put(
            self.root,
            "godot/depot-test-assets.txt",
            "# core\nmodels/boar.m2\n\nLight.csv  # lighting\n",
        )
        self._put(self.root, "godot/.godot/imported.rs", "private")
        self._put(self.root, "target/debug/hidden.rs", "private")
        self._put(self.root, ".env", "token")
        self._put(self.root, ".gitignore", "*.ignored.rs\n")
        self._put(self.root, "godot/core/src/deleted.rs", "deleted")
        self._git(self.root, "init", "-q")
        self._git(self.root, "add", ".")
        self._git(self.root, "commit", "-qm", "initial")
        self._put(self.root, "godot/rust/src/lib.rs", "modified")
        (self.root / "godot/core/src/deleted.rs").unlink()
        self._put(self.root, "godot/core/src/new.rs", "untracked")
        self._put(self.root, "godot/rust/secrets.toml", "password=secret")
        self._put(self.root, "godot/core/src/skip.ignored.rs", "ignored")
        for name in SIBLINGS:
            repo = self.base / name
            repo.mkdir()
            self._put(
                repo, "Cargo.toml", "[package]\nname='fixture'\nversion='0.1.0'\n"
            )
            self._put(repo, "src/lib.rs", name)
            self._git(repo, "init", "-q")
            self._git(repo, "add", ".")
            self._git(repo, "commit", "-qm", "initial")
        bin_dir = self.base / "bin"
        bin_dir.mkdir()
        depot = bin_dir / "docker"
        depot.write_text(FAKE_DEPOT)
        depot.chmod(0o755)
        self.record = self.base / "record.jsonl"
        self.home = self.base / "home"
        self._put(self.home, ".config/game-engine/build-host", "local\n")
        self.env = {
            key: value
            for key, value in os.environ.items()
            if key not in ("XDG_CONFIG_HOME", "DEPOT_TOKEN")
        }
        self.env.update(
            PATH=str(bin_dir) + os.pathsep + os.environ["PATH"],
            HOME=str(self.home),
            XDG_CACHE_HOME=str(self.base / "cache"),
            DEPOT_RECORD=str(self.record),
        )

    def _put(self, root, name, content):
        path = root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content)

    def _git(self, root, *args):
        subprocess.run(
            [
                "git",
                "-C",
                str(root),
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.invalid",
                *args,
            ],
            check=True,
            capture_output=True,
        )

    def build(self, *, fixture=None, cli=False, **env):
        command = ["python3", str(SCRIPT), "--root", str(self.root)]
        if fixture is not None:
            command.extend(["--fixture", fixture])
        if cli:
            command.append("--cli")
        return subprocess.run(
            command,
            env={**self.env, **env},
            text=True,
            capture_output=True,
            check=False,
        )

    def run_test_mode(self, *cargo_args, **env):
        command = [
            "python3",
            str(SCRIPT),
            "--root",
            str(self.root),
            "--test",
            *cargo_args,
        ]
        return subprocess.run(
            command,
            env={**self.env, **env},
            text=True,
            capture_output=True,
            check=False,
        )

    def records(self):
        return (
            [json.loads(line) for line in self.record.read_text().splitlines()]
            if self.record.exists()
            else []
        )

    def build_args(self, record):
        args = record["args"]
        return dict(
            args[index + 1].split("=", 1)
            for index, arg in enumerate(args)
            if arg == "--build-arg"
        )

    def test_saved_host_builds_without_depot_credentials(self):
        result = self.build()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.records()[0]["project"], "game-engine")

    def test_save_host_only_writes_setting_without_building(self):
        result = subprocess.run(
            ["python3", str(SCRIPT), "--save-build-host", "desktop"],
            env=self.env,
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(
            (self.home / ".config/game-engine/build-host").read_text(), "desktop\n"
        )
        self.assertEqual(self.records(), [])

    def test_explicit_local_overrides_saved_desktop(self):
        self._put(self.home, ".config/game-engine/build-host", "desktop\n")
        result = subprocess.run(
            ["python3", str(SCRIPT), "--root", str(self.root), "--build-host", "local"],
            env=self.env,
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(
            (self.root / "target/debug/libgame_engine_godot.so").read_bytes(),
            b"new binary",
        )
        self.assertEqual(
            (self.home / ".config/game-engine/build-host").read_text(), "desktop\n"
        )

    def test_invalid_or_missing_saved_host_requires_explicit_selection(self):
        setting = self.home / ".config/game-engine/build-host"
        setting.write_text("depot\n")
        result = self.build()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("build-host", result.stderr)
        setting.unlink()
        result = self.build()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("--build-host", result.stderr)
        self.assertEqual(self.records(), [])

    def test_source_snapshot_and_originating_install(self):
        result = self.build(DEPOT_PROJECT_ID="custom-id")
        self.assertEqual(result.returncode, 0, result.stderr)
        snapshot = self.records()[0]
        files = snapshot["files"]
        prefix = "game-engine-godot-conversion/"
        self.assertEqual(snapshot["project"], "game-engine")
        self.assertEqual(
            snapshot["args"][snapshot["args"].index("--platform") + 1], "linux/amd64"
        )
        self.assertEqual(files[prefix + "godot/rust/src/lib.rs"], "modified")
        self.assertEqual(files[prefix + "godot/core/src/new.rs"], "untracked")
        self.assertEqual(
            files[
                prefix + "godot/rust/src/rendering/ui/nameplate_skins/health-fill.png"
            ],
            "png",
        )
        self.assertNotIn(prefix + "godot/core/src/deleted.rs", files)
        self.assertEqual(
            files[prefix + "vendor/taffy/README.md"], "required compile include"
        )
        for crate in ("taffy", "ktx2-rw"):
            self.assertEqual(
                files[prefix + f"vendor/{crate}/src/lib.rs"], "vendored patch"
            )
        self.assertFalse(any(key.startswith("bevy-patches/") for key in files))
        for name in SIBLINGS:
            self.assertIn(name + "/src/lib.rs", files)
        for excluded in (
            "data/private.rs",
            "godot/.godot/imported.rs",
            "target/debug/hidden.rs",
            ".env",
            "godot/core/src/skip.ignored.rs",
            "godot/rust/secrets.toml",
            "src/legacy.rs",
        ):
            self.assertNotIn(prefix + excluded, files)
        self.assertTrue(all(not key.startswith("/") for key in files))
        artifact = self.root / "target/debug/libgame_engine_godot.so"
        self.assertEqual(artifact.read_bytes(), b"new binary")
        self.assertIn(str(artifact), result.stdout)
        self.assertFalse(Path(snapshot["context"]).exists())

    def test_ui_toolkit_snapshot_uses_canonical_sibling(self):
        self._put(self.base / "ui-toolkit", "core/Cargo.toml", "[package]\n")
        self._put(self.base / "ui-toolkit", "core/src/lib.rs", "portable core")
        result = self.build()
        self.assertEqual(result.returncode, 0, result.stderr)
        files = self.records()[0]["files"]
        self.assertEqual(files["ui-toolkit/src/lib.rs"], "ui-toolkit")
        self.assertEqual(files["ui-toolkit/core/Cargo.toml"], "[package]\n")
        self.assertEqual(files["ui-toolkit/core/src/lib.rs"], "portable core")
        self.assertFalse(
            any(key.startswith("ui-toolkit-godot-conversion/") for key in files)
        )

    def test_ui_toolkit_snapshot_uses_canonical_override(self):
        override = self.base / "toolkit-branch"
        (self.base / "ui-toolkit").rename(override)
        self._put(override, "src/lib.rs", "override toolkit")
        result = self.build(DEPOT_SIBLING_UI_TOOLKIT=str(override))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(
            self.records()[0]["files"]["ui-toolkit/src/lib.rs"], "override toolkit"
        )

    def test_ui_toolkit_snapshot_rejects_legacy_override(self):
        legacy = self.base / "ui-toolkit-godot-conversion"
        (self.base / "ui-toolkit").rename(legacy)
        result = self.build(DEPOT_SIBLING_UI_TOOLKIT_GODOT_CONVERSION=str(legacy))
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(
            f"missing build dependency: {self.base / 'ui-toolkit'}", result.stderr
        )
        self.assertEqual(self.records(), [])

    def test_requested_fixture_installs_only_named_executable_and_extension(self):
        for name in FIXTURES:
            result = self.build(fixture=name, DEPOT_FIXTURE=name)
            self.assertEqual(result.returncode, 0, result.stderr)
            executable = self.root / "target/debug/examples" / name
            self.assertEqual(executable.read_bytes(), name.encode())
            self.assertTrue(os.access(executable, os.X_OK))
            self.assertEqual(
                (self.root / "target/debug/libgame_engine_godot.so").read_bytes(),
                b"new binary",
            )
            self.assertNotIn("target/", "".join(self.records()[-1]["files"]))
            self.assertEqual(self.build_args(self.records()[-1])["FIXTURE"], name)
        other = self.root / "target/debug/examples/not_requested"
        self.assertFalse(other.exists())
        result = self.build()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertNotIn("FIXTURE", self.build_args(self.records()[-1]))

    def test_cli_installs_executable_alongside_fixture_and_extension(self):
        result = self.build(fixture="native_input_fixture", cli=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        cli = self.root / "target/debug/game-engine-cli"
        self.assertEqual(cli.read_bytes(), b"cli binary")
        self.assertTrue(os.access(cli, os.X_OK))
        self.assertEqual(
            (self.root / "target/debug/examples/native_input_fixture").read_bytes(),
            b"fixture binary",
        )
        self.assertEqual(self.build_args(self.records()[-1])["CLI"], "1")
        self.assertIn(str(cli), result.stdout)
        cli.unlink()
        result = self.build()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertNotIn("CLI", self.build_args(self.records()[-1]))
        self.assertFalse(cli.exists())

    def test_release_installs_optimized_library_only_under_target_release(self):
        command = ["python3", str(SCRIPT), "--root", str(self.root), "--release"]
        result = subprocess.run(
            command,
            env={**self.env, "DEPOT_ARTIFACT": "release binary"},
            text=True,
            capture_output=True,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        release = self.root / "target/release/libgame_engine_godot.so"
        self.assertEqual(release.read_bytes(), b"release binary")
        self.assertIn(str(release), result.stdout)
        self.assertFalse((self.root / "target/debug/libgame_engine_godot.so").exists())
        self.assertEqual(self.build_args(self.records()[-1])["RELEASE"], "1")
        result = self.build()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertNotIn("RELEASE", self.build_args(self.records()[-1]))
        self.assertEqual(release.read_bytes(), b"release binary")

    def test_release_is_not_allowed_with_fixture_test_or_cli(self):
        for extra in (["--fixture", "native_input_fixture"], ["--test"], ["--cli"]):
            command = [
                "python3",
                str(SCRIPT),
                "--root",
                str(self.root),
                "--release",
                *extra,
            ]
            result = subprocess.run(
                command, env=self.env, text=True, capture_output=True, check=False
            )
            self.assertNotEqual(result.returncode, 0, extra)
            self.assertIn("not allowed", result.stderr)
        self.assertEqual(self.records(), [])

    def test_cli_is_not_allowed_with_test_mode(self):
        command = ["python3", str(SCRIPT), "--root", str(self.root), "--cli", "--test"]
        result = subprocess.run(
            command, env=self.env, text=True, capture_output=True, check=False
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("not allowed", result.stderr)
        self.assertEqual(self.records(), [])

    def test_test_mode_forwards_cargo_args_stages_listed_assets_and_prints_log(self):
        result = self.run_test_mode(
            "-p",
            "game-engine-core",
            "--test",
            "m2_events",
            "--",
            "--exact",
            "a b",
            DEPOT_TEST_LOG="3 passed",
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("test result: 3 passed", result.stdout)
        saved = self.root / "target/depot-test.log"
        self.assertIn(f"Full log: {saved}", result.stdout)
        self.assertEqual(saved.read_text(), "test result: 3 passed\n")
        build, staged = self.records()
        args = build["args"]
        self.assertEqual(args[args.index("--target") + 1], "test-result")
        self.assertEqual(
            self.build_args(build)["TEST_ARGS"],
            "-p game-engine-core --test m2_events -- --exact 'a b'",
        )
        self.assertTrue(
            self.build_args(build)["TARGET_CACHE"].startswith("godot-target-")
        )
        self.assertEqual(
            staged["assets"],
            {"models/boar.m2": "boar model", "Light.csv": "light rows"},
        )
        self.assertFalse(
            any(
                name.startswith("game-engine-godot-conversion/data/")
                for name in build["files"]
            )
        )
        self.assertFalse(Path(staged["asset_dir"]).exists())
        self.assertFalse((self.root / "target/debug/libgame_engine_godot.so").exists())

    def test_context_path_is_stable_per_checkout_and_mode(self):
        for _ in range(2):
            self.assertEqual(self.run_test_mode().returncode, 0)
            self.assertEqual(self.build().returncode, 0)
        contexts = [record["context"] for record in self.records() if "args" in record]
        test_contexts, build_contexts = contexts[0::2], contexts[1::2]
        self.assertEqual(len(set(test_contexts)), 1)
        self.assertEqual(len(set(build_contexts)), 1)
        self.assertNotEqual(test_contexts[0], build_contexts[0])
        other = self.base / "game-engine-alt-worktree"
        self._git(self.base, "clone", "-q", str(self.root), str(other))
        self._put(other, "data/models/boar.m2", "boar model")
        self._put(other, "data/Light.csv", "light rows")
        result = subprocess.run(
            ["python3", str(SCRIPT), "--root", str(other), "--test"],
            env=self.env,
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertNotEqual(self.records()[-2]["context"], test_contexts[0])
        self.assertFalse(Path(test_contexts[0]).exists())

    def test_test_mode_exits_with_cargo_status(self):
        result = self.run_test_mode(
            "-p", "game-engine-core", DEPOT_TEST_STATUS="101", DEPOT_TEST_LOG="1 failed"
        )
        self.assertEqual(result.returncode, 101)
        self.assertIn("test result: 1 failed", result.stdout)
        result = self.run_test_mode(DEPOT_NO_LOG="1")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("test.log", result.stderr)
        result = self.run_test_mode(DEPOT_FAIL="1")
        self.assertNotEqual(result.returncode, 0)

    def test_test_mode_restages_changed_and_removed_assets(self):
        first = self.run_test_mode()
        self.assertEqual(first.returncode, 0, first.stderr)
        self._put(self.root, "data/models/boar.m2", "boar model v2")
        os.utime(self.root / "data/models/boar.m2", ns=(1_800_000_000_000_000_000,) * 2)
        self._put(self.root, "godot/depot-test-assets.txt", "models/boar.m2\n")
        second = self.run_test_mode()
        self.assertEqual(second.returncode, 0, second.stderr)
        staged = [record for record in self.records() if "assets" in record]
        self.assertEqual(staged[1]["assets"], {"models/boar.m2": "boar model v2"})

    def test_restaging_a_replaced_asset_never_writes_through_to_the_old_file(self):
        first = self.run_test_mode()
        self.assertEqual(first.returncode, 0, first.stderr)
        source = self.root / "data/models/boar.m2"
        kept = self.base / "kept-old-boar.m2"
        os.link(source, kept)
        source.unlink()
        source.write_text("replacement boar")
        os.utime(source, ns=(1_900_000_000_000_000_000,) * 2)
        second = self.run_test_mode()
        self.assertEqual(second.returncode, 0, second.stderr)
        self.assertEqual(kept.read_text(), "boar model")
        self.assertEqual(
            self.records()[-1]["assets"]["models/boar.m2"], "replacement boar"
        )

    def test_test_mode_rejects_missing_or_escaping_assets_before_depot(self):
        for listed, message in (
            ("models/absent.m2", "data/models/absent.m2"),
            ("../.env", "inside data/"),
        ):
            self._put(self.root, "godot/depot-test-assets.txt", listed + "\n")
            result = self.run_test_mode("-p", "game-engine-core")
            self.assertNotEqual(result.returncode, 0)
            self.assertIn(message, result.stderr)
        self.assertEqual(self.records(), [])

    def test_test_and_fixture_modes_are_exclusive(self):
        command = [
            "python3",
            str(SCRIPT),
            "--root",
            str(self.root),
            "--fixture",
            "native_input_fixture",
            "--test",
        ]
        result = subprocess.run(
            command, env=self.env, text=True, capture_output=True, check=False
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("not allowed", result.stderr)

    def test_bad_fixture_choice_fails_before_depot(self):
        for name in ("../../other", "fixture_support", "native_input_fixture/swimming"):
            result = self.build(fixture=name)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("unknown fixture", result.stderr)
        self.assertEqual(self.records(), [])

    def test_new_example_file_is_buildable_without_script_changes(self):
        self._put(
            self.root, "godot/network/examples/native_water_fixture.rs", "fn main() {}"
        )
        result = self.build(fixture="native_water_fixture", DEPOT_FIXTURE="water")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(
            (self.root / "target/debug/examples/native_water_fixture").read_bytes(),
            b"water",
        )

    def test_isolated_runtime_xdg_config_does_not_change_saved_build_host(self):
        isolated = self.base / "fixture-config"
        isolated.mkdir()
        result = self.build(XDG_CONFIG_HOME=str(isolated))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(
            (self.root / "target/debug/libgame_engine_godot.so").read_bytes(),
            b"new binary",
        )

    def test_failed_fixture_download_preserves_existing_executable(self):
        executable = self.root / "target/debug/examples/native_input_fixture"
        executable.parent.mkdir(parents=True, exist_ok=True)
        executable.write_bytes(b"old fixture")
        executable.chmod(0o755)
        for failure in ({"DEPOT_MISSING_FIXTURE": "1"}, {"DEPOT_CORRUPT_FIXTURE": "1"}):
            result = self.build(fixture="native_input_fixture", **failure)
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(executable.read_bytes(), b"old fixture")
            self.assertTrue(os.access(executable, os.X_OK))

    def test_fixture_install_is_checkout_local(self):
        other = self.base / "game-engine-alt-worktree"
        self._git(self.base, "clone", "-q", str(self.root), str(other))
        first = self.build(fixture="native_input_fixture", DEPOT_FIXTURE="first")
        self.assertEqual(first.returncode, 0, first.stderr)
        second = subprocess.run(
            [
                "python3",
                str(SCRIPT),
                "--root",
                str(other),
                "--fixture",
                "native_npc_visual_fixture",
            ],
            env={**self.env, "DEPOT_FIXTURE": "second"},
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(second.returncode, 0, second.stderr)
        self.assertEqual(
            (self.root / "target/debug/examples/native_input_fixture").read_bytes(),
            b"first",
        )
        self.assertEqual(
            (other / "target/debug/examples/native_npc_visual_fixture").read_bytes(),
            b"second",
        )
        self.assertFalse(
            (other / "target/debug/examples/native_input_fixture").exists()
        )

    def test_source_mtimes_survive_snapshots_with_working_tree_edits(self):
        prefix = "game-engine-godot-conversion/"
        unchanged = self.root / "godot/core/src/asset/mod.rs"
        edited = self.root / "godot/rust/src/lib.rs"
        untracked = self.root / "godot/core/src/new.rs"
        timestamps = (
            1_700_000_000_123_456_789,
            1_700_000_001_123_456_789,
            1_700_000_002_123_456_789,
        )
        for source, timestamp in zip((unchanged, edited, untracked), timestamps):
            os.utime(source, ns=(timestamp, timestamp))
        first = self.build()
        self.assertEqual(first.returncode, 0, first.stderr)
        edited.write_text("edited after first snapshot")
        os.utime(
            edited, ns=(timestamps[1] + 1_000_000_000, timestamps[1] + 1_000_000_000)
        )
        second = self.build()
        self.assertEqual(second.returncode, 0, second.stderr)
        snapshots = self.records()
        for index in (0, 1):
            self.assertEqual(
                snapshots[index]["mtimes"][prefix + "godot/core/src/asset/mod.rs"],
                timestamps[0],
            )
            self.assertEqual(
                snapshots[index]["mtimes"][prefix + "godot/core/src/new.rs"],
                timestamps[2],
            )
        self.assertEqual(
            snapshots[0]["mtimes"][prefix + "godot/rust/src/lib.rs"], timestamps[1]
        )
        self.assertEqual(
            snapshots[1]["mtimes"][prefix + "godot/rust/src/lib.rs"],
            timestamps[1] + 1_000_000_000,
        )
        self.assertEqual(
            snapshots[1]["files"][prefix + "godot/rust/src/lib.rs"],
            "edited after first snapshot",
        )

    def test_remote_refresh_is_shipped_and_makes_old_inputs_newer_without_touching_cache(
        self,
    ):
        result = self.build()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("refresh-source-mtimes.py", self.records()[0]["files"])

        context = self.base / "remote-context"
        source = context / "game-engine-godot-conversion/godot/core/src/asset/mod.rs"
        manifest = context / "game-engine-godot-conversion/godot/Cargo.toml"
        included = context / "game-engine-godot-conversion/vendor/taffy/README.md"
        texture = (
            context
            / "game-engine-godot-conversion/godot/rust/src/rendering/ui/nameplate_skins/health-fill.png"
        )
        cache = context / "game-engine-godot-conversion/target/debug/libcore.rlib"
        top_cache = context / "target/debug/libcore.rlib"
        registry = context / "game-engine-godot-conversion/godot/.cache/generated.rs"
        for path in (source, manifest, included, texture, cache, top_cache, registry):
            self._put(context, str(path.relative_to(context)), "fixture")
        old = 1_600_000_000_000_000_000
        for path in (source, manifest, included, texture, cache, top_cache, registry):
            os.utime(path, ns=(old, old))
        outside = self.base / "outside.rs"
        outside.write_text("outside")
        os.utime(outside, ns=(old, old))
        (source.parent / "link.rs").symlink_to(outside)
        failed = subprocess.run(
            ["python3", str(REFRESH), str(context)],
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertNotEqual(failed.returncode, 0)
        self.assertIn("symlink", failed.stderr.lower())
        self.assertEqual(outside.stat().st_mtime_ns, old)
        (source.parent / "link.rs").unlink()
        refreshed = subprocess.run(
            ["python3", str(REFRESH), str(context)],
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(refreshed.returncode, 0, refreshed.stderr)
        for path in (source, manifest, included, texture):
            self.assertGreater(path.stat().st_mtime_ns, old)
            self.assertEqual(path.read_text(), "fixture")
        for path in (cache, top_cache, registry, outside):
            self.assertEqual(path.stat().st_mtime_ns, old)

    def test_failed_build_and_corrupt_gzip_preserve_existing_artifact(self):
        artifact = self.root / "target/debug/libgame_engine_godot.so"
        artifact.parent.mkdir(parents=True, exist_ok=True)
        artifact.write_bytes(b"old binary")
        for failure in ({"DEPOT_FAIL": "1"}, {"DEPOT_CORRUPT": "1"}):
            result = self.build(**failure)
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(artifact.read_bytes(), b"old binary")
        for snapshot in self.records():
            self.assertFalse(Path(snapshot["context"]).exists())

    def test_missing_dependency_and_source_symlink_fail_before_depot(self):
        (self.base / "ui-toolkit-macros/Cargo.toml").unlink()
        result = self.build()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("ui-toolkit-macros/Cargo.toml", result.stderr)
        self.assertEqual(self.records(), [])
        self._put(self.base / "ui-toolkit-macros", "Cargo.toml", "restored")
        (self.root / "godot/core/Cargo.toml").unlink()
        result = self.build()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("godot/core/Cargo.toml", result.stderr)
        self.assertEqual(self.records(), [])
        self._put(self.root, "godot/core/Cargo.toml", "restored")
        (self.root / "godot/core/src/new.rs").unlink()
        (self.root / "godot/core/src/new.rs").symlink_to("asset/mod.rs")
        result = self.build()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("symlink", result.stderr.lower())
        self.assertEqual(self.records(), [])

    def test_two_physical_checkout_names_install_separately(self):
        other = self.base / "game-engine-alt-worktree"
        self._git(self.base, "clone", "-q", str(self.root), str(other))
        first = self.build(DEPOT_ARTIFACT="first")
        self.assertEqual(first.returncode, 0, first.stderr)
        second = subprocess.run(
            ["python3", str(SCRIPT), "--root", str(other)],
            env={**self.env, "DEPOT_ARTIFACT": "second"},
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(second.returncode, 0, second.stderr)
        self.assertEqual(
            (self.root / "target/debug/libgame_engine_godot.so").read_bytes(), b"first"
        )
        self.assertEqual(
            (other / "target/debug/libgame_engine_godot.so").read_bytes(), b"second"
        )
        self.assertEqual(
            self.records()[1]["files"][
                "game-engine-godot-conversion/godot/rust/src/lib.rs"
            ],
            "original",
        )

    def test_target_cache_is_per_checkout_and_stable_with_optional_fixture(self):
        other = self.base / "game-engine-alt-worktree"
        self._git(self.base, "clone", "-q", str(self.root), str(other))
        first = self.build()
        second = self.build(fixture="native_input_fixture")
        third = subprocess.run(
            [
                "python3",
                str(SCRIPT),
                "--root",
                str(other),
                "--fixture",
                "native_npc_visual_fixture",
            ],
            env=self.env,
            capture_output=True,
            text=True,
            check=False,
        )
        for result in (first, second, third):
            self.assertEqual(result.returncode, 0, result.stderr)
        defaults, fixture, alternate = [
            self.build_args(record) for record in self.records()
        ]
        self.assertTrue(defaults["TARGET_CACHE"].startswith("godot-target-"), defaults)
        self.assertEqual(defaults["TARGET_CACHE"], fixture["TARGET_CACHE"])
        self.assertNotEqual(defaults["TARGET_CACHE"], alternate["TARGET_CACHE"])
        self.assertNotIn("FIXTURE", defaults)
        self.assertEqual(fixture["FIXTURE"], "native_input_fixture")
        self.assertEqual(alternate["FIXTURE"], "native_npc_visual_fixture")
        self.assertEqual(
            (self.root / "target/debug/examples/native_input_fixture").read_bytes(),
            b"fixture binary",
        )
        self.assertEqual(
            (other / "target/debug/examples/native_npc_visual_fixture").read_bytes(),
            b"fixture binary",
        )

    def test_target_directory_symlink_fails_without_remote_build(self):
        outside = self.base / "other-target"
        outside.mkdir()
        (self.root / "target").rename(self.root / "old-target")
        (self.root / "target").symlink_to(outside, target_is_directory=True)
        result = self.build()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("target symlink", result.stderr.lower())
        self.assertEqual(self.records(), [])

    def test_examples_directory_symlink_cannot_install_outside_checkout(self):
        outside = self.base / "other-examples"
        outside.mkdir()
        examples = self.root / "target/debug/examples"
        examples.parent.mkdir(parents=True, exist_ok=True)
        examples.symlink_to(outside, target_is_directory=True)
        result = self.build(fixture="native_input_fixture")
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse((outside / "native_input_fixture").exists())
        self.assertEqual(self.records(), [])

    def test_same_checkout_builds_serialize_install(self):
        cmd = ["python3", str(SCRIPT), "--root", str(self.root)]
        first = subprocess.Popen(
            cmd,
            env={**self.env, "DEPOT_DELAY": "0.5", "DEPOT_ARTIFACT": "first"},
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        try:
            deadline = time.monotonic() + 10
            while not self.records() and time.monotonic() < deadline:
                time.sleep(0.02)
            self.assertTrue(self.records(), "first build did not reach Depot")
            second = subprocess.run(
                cmd,
                env={**self.env, "DEPOT_ARTIFACT": "second"},
                capture_output=True,
                text=True,
                timeout=15,
                check=False,
            )
            _, stderr = first.communicate(timeout=15)
            self.assertEqual(first.returncode, 0, stderr)
            self.assertEqual(second.returncode, 0, second.stderr)
            self.assertEqual(
                (self.root / "target/debug/libgame_engine_godot.so").read_bytes(),
                b"second",
            )
        finally:
            if first.poll() is None:
                first.kill()
            first.communicate()


if __name__ == "__main__":
    unittest.main()
