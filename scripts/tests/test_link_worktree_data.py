"""Behavioral fixtures for shared assets and idle-slot data repair."""

import contextlib
import importlib.util
import io
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch


SCRIPT = Path(__file__).resolve().parents[1] / "agent/link-worktree-data.py"


class LinkWorktreeDataTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.base = Path(temporary.name)
        self.canonical = self.base / "canonical"
        self.worktree = self.base / "slot"
        self.canonical.mkdir()
        self.worktree.mkdir()
        subprocess.run(["git", "init", "-q", str(self.worktree)], check=True)
        self.proc = self.base / "proc"
        self.proc.mkdir()

    def put(self, root, relative, content):
        path = root / "data" / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content)
        return path

    def link(self, repair=False):
        spec = importlib.util.spec_from_file_location("data_links", SCRIPT)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        output = io.StringIO()
        args = [str(self.canonical), str(self.worktree)]
        if repair:
            args.append("--repair")
        with patch.object(module, "PROC_ROOT", self.proc):
            with contextlib.redirect_stdout(output), contextlib.redirect_stderr(output):
                result = module.main(args)
        return result, output.getvalue()

    def test_fresh_directory_links_share_later_assets_and_writes(self):
        directories = (
            "textures", "models", "terrain", "dbfilesclient",
            "db2", "sounds", "music", "cache",
        )
        for directory in directories:
            self.put(self.canonical, f"{directory}/seed", "seed")
        result = subprocess.run(
            ["python3", str(SCRIPT), str(self.canonical), str(self.worktree)],
            capture_output=True, text=True, check=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        for directory in directories:
            self.assertTrue((self.worktree / "data" / directory).is_symlink())
            self.put(self.canonical, f"{directory}/later", "new canonical asset")
            self.assertEqual(
                (self.worktree / "data" / directory / "later").read_text(),
                "new canonical asset",
            )
            self.put(self.worktree, f"{directory}/extracted", "slot write")
            self.assertEqual(
                (self.canonical / "data" / directory / "extracted").read_text(),
                "slot write",
            )
        self.assertEqual(self.link()[0], 0)

    def test_repair_moves_extras_deduplicates_and_preserves_conflicts(self):
        self.put(self.canonical, "textures/duplicate", "same")
        self.put(self.canonical, "textures/conflict", "canonical")
        self.put(self.worktree, "textures/duplicate", "same")
        self.put(self.worktree, "textures/conflict", "slot version")
        self.put(self.worktree, "textures/nested/extra", "extracted")
        (self.worktree / "data/textures/legacy").symlink_to(
            self.canonical / "data/textures/duplicate"
        )
        result, output = self.link(repair=True)
        self.assertEqual(result, 0, output)
        self.assertTrue((self.worktree / "data/textures").is_symlink())
        self.assertEqual(
            (self.canonical / "data/textures/nested/extra").read_text(), "extracted"
        )
        self.assertEqual(
            (self.canonical / "data/textures/conflict").read_text(), "canonical"
        )
        self.assertEqual((self.canonical / "data/textures/legacy").read_text(), "same")
        self.assertEqual(list(self.worktree.glob("*repair-conflicts*")), [])
        backups = list((self.worktree.parent / "repair-conflicts").glob(
            f"{self.worktree.name}-data-repair-conflicts-*"
        ))
        self.assertEqual(len(backups), 1)
        self.assertEqual((backups[0] / "textures/conflict").read_text(), "slot version")
        self.assertIn("conflict", output.lower())
        self.assertIn(str(backups[0]), output)

    def test_tracked_subtrees_stay_local_but_untracked_children_are_shared(self):
        tracked = self.put(self.worktree, "ui/authored", "branch art")
        subprocess.run(
            ["git", "-C", str(self.worktree), "add", "-f", "data/ui/authored"],
            check=True,
        )
        self.put(self.canonical, "ui/authored", "canonical art")
        self.put(self.canonical, "ui/generated/icon", "generated")
        self.put(self.canonical, "auth_token.private", "secret")
        self.put(self.canonical, "root.sqlite-wal", "sidecar")
        result, output = self.link(repair=True)
        self.assertEqual(result, 0, output)
        self.assertFalse((self.worktree / "data/ui").is_symlink())
        self.assertEqual(tracked.read_text(), "branch art")
        self.assertTrue((self.worktree / "data/ui/generated").is_symlink())
        self.assertFalse((self.worktree / "data/auth_token.private").exists())
        self.assertFalse((self.worktree / "data/root.sqlite-wal").exists())

    def test_real_directory_requires_explicit_repair(self):
        self.put(self.canonical, "textures/existing", "canonical")
        extra = self.put(self.worktree, "textures/extra", "slot")
        result = subprocess.run(
            ["python3", str(SCRIPT), str(self.canonical), str(self.worktree)],
            capture_output=True, text=True, check=False,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("--repair", result.stderr)
        self.assertEqual(extra.read_text(), "slot")
        self.assertFalse((self.canonical / "data/textures/extra").exists())

    def test_open_file_blocks_entire_repair_before_any_changes(self):
        extra = self.put(self.worktree, "textures/extra", "slot")
        self.put(self.worktree, "models/extra", "model")
        self.put(self.canonical, "textures/seed", "seed")
        self.put(self.canonical, "models/seed", "seed")
        # Use real kernel descriptor links; unrelated host processes may be unreadable.
        (self.proc / str(os.getpid())).symlink_to(Path("/proc") / str(os.getpid()))
        with extra.open():
            result, output = self.link(repair=True)
        self.assertNotEqual(result, 0)
        self.assertIn(str(os.getpid()), output)
        self.assertIn(str(extra), output)
        self.assertFalse((self.canonical / "data/models/extra").exists())
        self.assertEqual(extra.read_text(), "slot")
        self.assertEqual(self.link(repair=True)[0], 0)

    def test_retired_isolation_cannot_remove_shared_assets(self):
        shared = self.put(self.canonical, "textures/asset", "keep")
        self.link()
        old_script = SCRIPT.with_name("first-use-data.py")
        for action in ("isolate", "reset"):
            result = subprocess.run(
                ["python3", str(old_script), str(self.worktree), action],
                capture_output=True, text=True, check=False,
            )
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("retired", result.stderr)
            self.assertTrue((self.worktree / "data/textures").is_symlink())
            self.assertEqual(shared.read_text(), "keep")

    def test_unreadable_descriptors_are_named_and_skipped(self):
        # Non-dumpable services (systemd --user) hide their descriptors from their owner.
        self.put(self.canonical, "textures/seed", "seed")
        self.put(self.worktree, "textures/extra", "slot")
        process = self.proc / "123456"
        (process / "fd").mkdir(parents=True)
        (process / "comm").write_text("systemd\n")
        (process / "fd").chmod(0)
        self.addCleanup((process / "fd").chmod, 0o700)
        result, output = self.link(repair=True)
        self.assertEqual(result, 0, output)
        self.assertIn("skipped unreadable PID 123456 (systemd)", output)
        self.assertEqual((self.canonical / "data/textures/extra").read_text(), "slot")
        self.assertTrue((self.worktree / "data/textures").is_symlink())

    def test_slot_only_directory_is_moved_to_canonical(self):
        (self.canonical / "data").mkdir()
        self.put(self.worktree, "new-assets/extra", "slot")
        result, output = self.link(repair=True)
        self.assertEqual(result, 0, output)
        self.assertTrue((self.worktree / "data/new-assets").is_symlink())
        self.assertEqual((self.canonical / "data/new-assets/extra").read_text(), "slot")


if __name__ == "__main__":
    unittest.main()
