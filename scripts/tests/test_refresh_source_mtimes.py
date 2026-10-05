"""Exercise source restaging against persistent Cargo timestamp state."""

import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


REFRESH = Path(__file__).resolve().parents[1] / "depot/refresh-source-mtimes.py"
OLD = 1_600_000_000_000_000_000


class RefreshSourceMtimesTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.base = Path(temporary.name)
        self.context = self.base / "context"
        self.target = self.context / "engine/target"
        self.target.mkdir(parents=True)
        self.relative = Path("engine/src/lib.rs")
        self.stage(self.relative, "pub const VALUE: u8 = 1;")

    def stage(self, relative, content):
        path = self.context / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content)
        os.utime(path, ns=(OLD, OLD))
        return path

    def refresh(self):
        result = subprocess.run(
            ["python3", str(REFRESH), str(self.context), str(self.target)],
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_identical_restaged_sources_keep_their_compiled_timestamps(self):
        inputs = {
            self.relative: "pub const VALUE: u8 = 1;",
            Path("toolkit/src/lib.rs"): "pub fn layout() {}",
            Path("engine/vendor/taffy/README.md"): "included documentation",
            Path("engine/src/icon.png"): "included image",
            Path("engine/Cargo.toml"): "[workspace]",
        }
        for relative, content in inputs.items():
            self.stage(relative, content)
        artifact = self.stage(Path("engine/target/debug/library.rlib"), "compiled")
        asset = self.stage(Path("engine/data/icon.png"), "asset")
        self.refresh()
        mtimes = {
            relative: (self.context / relative).stat().st_mtime_ns
            for relative in inputs
        }
        for relative in inputs:
            self.assertGreater(mtimes[relative], OLD)
        # BuildKit supplies a fresh source layer, but the target mount survives.
        shutil.rmtree(self.context / "toolkit")
        shutil.rmtree(self.context / "engine/src")
        for relative, content in inputs.items():
            self.stage(relative, content)
        self.refresh()
        self.assertEqual(
            {
                relative: (self.context / relative).stat().st_mtime_ns
                for relative in inputs
            },
            mtimes,
        )
        self.assertEqual(artifact.stat().st_mtime_ns, OLD)
        self.assertEqual(asset.stat().st_mtime_ns, OLD)

    def test_changed_content_with_old_mtime_and_reverts_are_invalidated(self):
        source = self.context / self.relative
        unchanged = self.stage(Path("engine/src/other.rs"), "pub fn other() {}")
        self.refresh()
        first = source.stat().st_mtime_ns
        stable = unchanged.stat().st_mtime_ns
        self.stage(self.relative, "pub const VALUE: u8 = 2;")
        self.refresh()
        second = source.stat().st_mtime_ns
        self.assertGreater(second, first)
        self.assertEqual(unchanged.stat().st_mtime_ns, stable)
        self.stage(self.relative, "pub const VALUE: u8 = 1;")
        self.refresh()
        self.assertGreater(source.stat().st_mtime_ns, second)
        self.assertEqual(unchanged.stat().st_mtime_ns, stable)

    def test_removed_then_reintroduced_input_gets_a_new_timestamp(self):
        source = self.context / self.relative
        self.refresh()
        first = source.stat().st_mtime_ns
        source.unlink()
        self.refresh()
        self.stage(self.relative, "pub const VALUE: u8 = 1;")
        self.refresh()
        self.assertGreater(source.stat().st_mtime_ns, first)


if __name__ == "__main__":
    unittest.main()
