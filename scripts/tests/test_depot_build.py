"""Behavioral tests for source-only Depot builds without network or Cargo."""

import gzip
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time
import unittest


SCRIPT = Path(__file__).resolve().parents[1] / "depot-build.py"
SIBLINGS = ("asset-resolver", "ui-toolkit-godot-conversion", "ui-toolkit-macros", "shared-protocol")
FAKE_DEPOT = '''#!/usr/bin/env python3
import gzip, json, os, pathlib, sys, time
args = sys.argv[1:]
assert args[:1] == ['build'], args
project = args[args.index('--project') + 1]
output = pathlib.Path(args[args.index('--output') + 1].split('dest=', 1)[1])
context = pathlib.Path(args[-1])
files = {str(p.relative_to(context)): p.read_bytes().decode('latin1') for p in context.rglob('*') if p.is_file()}
mtimes = {str(p.relative_to(context)): p.stat().st_mtime_ns for p in context.rglob('*') if p.is_file()}
with open(os.environ['DEPOT_RECORD'], 'a') as record:
    record.write(json.dumps({'project': project, 'files': files, 'mtimes': mtimes, 'context': str(context), 'args': args}) + '\\n')
if os.environ.get('DEPOT_DELAY'):
    time.sleep(float(os.environ['DEPOT_DELAY']))
if os.environ.get('DEPOT_FAIL'):
    sys.exit(7)
output.mkdir(parents=True, exist_ok=True)
artifact = output / 'libgame_engine_godot.so.gz'
artifact.write_bytes(b'bad gzip' if os.environ.get('DEPOT_CORRUPT') else gzip.compress(os.environ.get('DEPOT_ARTIFACT', 'new binary').encode()))
'''


class DepotBuildTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.base = Path(self.tmp.name)
        self.root = self.base / "game-engine-godot-conversion"
        self.root.mkdir()
        for name in ("godot", "src"):
            (self.root / name).mkdir()
        self._put(self.root, "godot/Cargo.toml", "[workspace]\nmembers=[]\n")
        self._put(self.root, "godot/Cargo.lock", "lock")
        for member in ("core", "network", "rust", "session", "ui-model"):
            self._put(self.root, f"godot/{member}/Cargo.toml", "[package]\nname='fixture'\nversion='0.1.0'\n")
        self._put(self.root, "godot/rust/src/lib.rs", "original")
        self._put(self.root, "src/asset/mod.rs", "asset")
        self._put(self.root, "src/rendering/ui/nameplate_skins/health-fill.png", "png")
        self._put(self.root, "data/private.rs", "private")
        self._put(self.root, "godot/.godot/imported.rs", "private")
        self._put(self.root, "target/debug/hidden.rs", "private")
        self._put(self.root, ".env", "token")
        self._put(self.root, ".gitignore", "*.ignored.rs\n")
        self._put(self.root, "src/deleted.rs", "deleted")
        self._git(self.root, "init", "-q")
        self._git(self.root, "add", ".")
        self._git(self.root, "commit", "-qm", "initial")
        self._put(self.root, "godot/rust/src/lib.rs", "modified")
        (self.root / "src/deleted.rs").unlink()
        self._put(self.root, "src/new.rs", "untracked")
        self._put(self.root, "godot/rust/secrets.toml", "password=secret")
        self._put(self.root, "src/skip.ignored.rs", "ignored")
        for name in SIBLINGS:
            repo = self.base / name
            repo.mkdir()
            self._put(repo, "Cargo.toml", "[package]\nname='fixture'\nversion='0.1.0'\n")
            self._put(repo, "src/lib.rs", name)
            self._git(repo, "init", "-q")
            self._git(repo, "add", ".")
            self._git(repo, "commit", "-qm", "initial")
        patch = self.base / "bevy-patches"
        patch.mkdir()
        for crate in ("taffy", "ktx2-rw"):
            self._put(patch, f"{crate}/Cargo.toml", "[package]\nname='fixture'\nversion='0.1.0'\n")
        self._put(patch, "taffy/README.md", "required compile include")
        self._put(patch, "taffy/src/lib.rs", "patch")
        self._git(patch, "init", "-q")
        self._git(patch, "add", ".")
        self._git(patch, "commit", "-qm", "initial")
        bin_dir = self.base / "bin"
        bin_dir.mkdir()
        depot = bin_dir / "depot"
        depot.write_text(FAKE_DEPOT)
        depot.chmod(0o755)
        self.record = self.base / "record.jsonl"
        self.env = dict(os.environ, PATH=str(bin_dir) + os.pathsep + os.environ["PATH"],
                        XDG_CACHE_HOME=str(self.base / "cache"), DEPOT_RECORD=str(self.record))

    def _put(self, root, name, content):
        path = root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content)

    def _git(self, root, *args):
        subprocess.run(["git", "-C", str(root), "-c", "user.name=Test", "-c", "user.email=test@example.invalid", *args], check=True, capture_output=True)

    def build(self, **env):
        return subprocess.run(["python3", str(SCRIPT), "--root", str(self.root)],
                              env={**self.env, **env}, text=True, capture_output=True)

    def records(self):
        return [json.loads(line) for line in self.record.read_text().splitlines()] if self.record.exists() else []

    def test_default_project_is_local_builds(self):
        self.env.pop("DEPOT_PROJECT_ID", None)
        result = self.build()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.records()[0]["project"], "003c4ttwqh")

    def test_source_snapshot_and_originating_install(self):
        result = self.build(DEPOT_PROJECT_ID="custom-id")
        self.assertEqual(result.returncode, 0, result.stderr)
        snapshot = self.records()[0]
        files = snapshot["files"]
        prefix = "game-engine-godot-conversion/"
        self.assertEqual(snapshot["project"], "custom-id")
        self.assertEqual(snapshot["args"][snapshot["args"].index("--platform") + 1], "linux/amd64")
        self.assertEqual(files[prefix + "godot/rust/src/lib.rs"], "modified")
        self.assertEqual(files[prefix + "src/new.rs"], "untracked")
        self.assertEqual(files[prefix + "src/rendering/ui/nameplate_skins/health-fill.png"], "png")
        self.assertNotIn(prefix + "src/deleted.rs", files)
        self.assertEqual(files["bevy-patches/taffy/README.md"], "required compile include")
        for name in SIBLINGS:
            self.assertIn(name + "/src/lib.rs", files)
        for excluded in ("data/private.rs", "godot/.godot/imported.rs", "target/debug/hidden.rs",
                         ".env", "src/skip.ignored.rs", "godot/rust/secrets.toml"):
            self.assertNotIn(prefix + excluded, files)
        self.assertTrue(all(not key.startswith("/") for key in files))
        artifact = self.root / "target/debug/libgame_engine_godot.so"
        self.assertEqual(artifact.read_bytes(), b"new binary")
        self.assertIn(str(artifact), result.stdout)
        self.assertFalse(Path(snapshot["context"]).exists())

    def test_source_mtimes_survive_snapshots_with_working_tree_edits(self):
        prefix = "game-engine-godot-conversion/"
        unchanged = self.root / "src/asset/mod.rs"
        edited = self.root / "godot/rust/src/lib.rs"
        untracked = self.root / "src/new.rs"
        timestamps = (1_700_000_000_123_456_789, 1_700_000_001_123_456_789,
                      1_700_000_002_123_456_789)
        for source, timestamp in zip((unchanged, edited, untracked), timestamps):
            os.utime(source, ns=(timestamp, timestamp))
        first = self.build()
        self.assertEqual(first.returncode, 0, first.stderr)
        edited.write_text("edited after first snapshot")
        os.utime(edited, ns=(timestamps[1] + 1_000_000_000, timestamps[1] + 1_000_000_000))
        second = self.build()
        self.assertEqual(second.returncode, 0, second.stderr)
        snapshots = self.records()
        for index in (0, 1):
            self.assertEqual(snapshots[index]["mtimes"][prefix + "src/asset/mod.rs"], timestamps[0])
            self.assertEqual(snapshots[index]["mtimes"][prefix + "src/new.rs"], timestamps[2])
        self.assertEqual(snapshots[0]["mtimes"][prefix + "godot/rust/src/lib.rs"], timestamps[1])
        self.assertEqual(snapshots[1]["mtimes"][prefix + "godot/rust/src/lib.rs"], timestamps[1] + 1_000_000_000)
        self.assertEqual(snapshots[1]["files"][prefix + "godot/rust/src/lib.rs"], "edited after first snapshot")

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
        (self.root / "src/new.rs").unlink()
        (self.root / "src/new.rs").symlink_to("asset/mod.rs")
        result = self.build()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("symlink", result.stderr.lower())
        self.assertEqual(self.records(), [])

    def test_two_physical_checkout_names_install_separately(self):
        other = self.base / "game-engine-alt-worktree"
        self._git(self.base, "clone", "-q", str(self.root), str(other))
        first = self.build(DEPOT_ARTIFACT="first")
        self.assertEqual(first.returncode, 0, first.stderr)
        second = subprocess.run(["python3", str(SCRIPT), "--root", str(other)],
                                env={**self.env, "DEPOT_ARTIFACT": "second"}, capture_output=True, text=True)
        self.assertEqual(second.returncode, 0, second.stderr)
        self.assertEqual((self.root / "target/debug/libgame_engine_godot.so").read_bytes(), b"first")
        self.assertEqual((other / "target/debug/libgame_engine_godot.so").read_bytes(), b"second")
        self.assertEqual(self.records()[1]["files"]["game-engine-godot-conversion/godot/rust/src/lib.rs"], "original")

    def test_target_directory_symlink_fails_without_remote_build(self):
        outside = self.base / "other-target"
        outside.mkdir()
        (self.root / "target").rename(self.root / "old-target")
        (self.root / "target").symlink_to(outside, target_is_directory=True)
        result = self.build()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("target symlink", result.stderr.lower())
        self.assertEqual(self.records(), [])

    def test_same_checkout_builds_serialize_install(self):
        cmd = ["python3", str(SCRIPT), "--root", str(self.root)]
        first = subprocess.Popen(cmd, env={**self.env, "DEPOT_DELAY": "0.5", "DEPOT_ARTIFACT": "first"}, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        try:
            deadline = time.monotonic() + 10
            while not self.records() and time.monotonic() < deadline:
                time.sleep(0.02)
            self.assertTrue(self.records(), "first build did not reach Depot")
            second = subprocess.run(cmd, env={**self.env, "DEPOT_ARTIFACT": "second"}, capture_output=True, text=True, timeout=15)
            stdout, stderr = first.communicate(timeout=15)
            self.assertEqual(first.returncode, 0, stderr)
            self.assertEqual(second.returncode, 0, second.stderr)
            self.assertEqual((self.root / "target/debug/libgame_engine_godot.so").read_bytes(), b"second")
        finally:
            if first.poll() is None:
                first.kill()
            first.communicate()


if __name__ == "__main__":
    unittest.main()
