import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "manifest_diff.py"


def write_manifest(path: Path, files: list[tuple[str, str, int]]) -> Path:
    entries = [{"path": p, "sha256": sha, "size": size} for p, sha, size in files]
    path.write_text(json.dumps({"version": "2026-10-15", "files": entries}))
    return path


class ManifestDiffTest(unittest.TestCase):
    def run_diff(self, live, new):
        with tempfile.TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            command = [sys.executable, str(SCRIPT), "--new", str(write_manifest(tmp / "new.json", new))]
            if live is not None:
                command += ["--live", str(write_manifest(tmp / "live.json", live))]
            command += ["--upload", str(tmp / "upload.txt"), "--delete", str(tmp / "delete.txt")]
            subprocess.run(command, check=True, capture_output=True)
            return (tmp / "upload.txt").read_text().splitlines(), (tmp / "delete.txt").read_text().splitlines()

    def test_uploads_changed_and_added_paths_and_deletes_removed_ones(self):
        live = [
            ("data/same.csv", "aa", 10),
            ("data/new-bytes.csv", "bb", 20),
            ("data/new-size.csv", "cc", 30),
            ("data/models/gone.m2", "dd", 40),
        ]
        new = [
            ("data/same.csv", "aa", 10),
            ("data/new-bytes.csv", "b2", 20),
            ("data/new-size.csv", "cc", 31),
            ("data/textures/added.blp", "ee", 50),
        ]
        upload, delete = self.run_diff(live, new)
        self.assertEqual(upload, ["data/new-bytes.csv", "data/new-size.csv", "data/textures/added.blp"])
        self.assertEqual(delete, ["data/models/gone.m2"])

    def test_first_publish_uploads_everything_and_deletes_nothing(self):
        upload, delete = self.run_diff(None, [("bin/game", "11", 1), ("data/a.csv", "22", 2)])
        self.assertEqual(upload, ["bin/game", "data/a.csv"])
        self.assertEqual(delete, [])

    def test_identical_bundle_publishes_nothing(self):
        files = [("data/a.csv", "22", 2)]
        self.assertEqual(self.run_diff(files, files), ([], []))


if __name__ == "__main__":
    unittest.main()
