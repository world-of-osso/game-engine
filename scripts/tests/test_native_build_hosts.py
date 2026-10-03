"""Bounded native transport process fixtures; no real cargo/host operations."""

import importlib
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

RUSTUP = """#!/usr/bin/env python3
import json, os, pathlib, sys
args = sys.argv[1:]
assert args[:2] == ['run', '1.98.1'], args
if args[2] == 'rustc':
    print(os.environ['FIXTURE_ROOT']); sys.exit(0)
assert args[2] == 'cargo' and '--locked' in args and '-j8' in args, args
root = pathlib.Path.cwd()
target = pathlib.Path(os.environ['CARGO_TARGET_DIR'])
target.mkdir(parents=True, exist_ok=True)
(target / 'observed.json').write_text(json.dumps({'cwd': str(root), 'args': args, 'setting': os.environ.get('SETTING')}))
profile = 'release' if '--release' in args else 'debug'
(target / profile).mkdir(exist_ok=True)
app = target / profile / 'fixture'
app.write_text('#!/usr/bin/env python3\\nimport os,sys\\nprint("runtime", os.getcwd(), os.environ.get("SETTING"), flush=True)\\nsys.exit(int(sys.argv[1]))\\n')
app.chmod(0o755)
sys.exit(int(os.environ.get('BUILD_STATUS', '0')))
"""


class NativeTests(unittest.TestCase):
    def setUp(self):
        self.assertTrue(
            (SCRIPTS / "native_build_hosts.py").exists(), "native transport missing"
        )
        self.h = importlib.import_module("native_build_hosts")
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name)
        self.context = self.base / "snapshot"
        (self.context / "project/src").mkdir(parents=True)
        (self.context / "project/src/main.rs").write_text("A")
        (self.context / "sibling").mkdir()
        (self.context / "sibling/lib.rs").write_text("sibling")
        self.bin = self.base / "bin"
        self.bin.mkdir()
        (self.bin / "rustup").write_text(RUSTUP)
        (self.bin / "rustup").chmod(0o755)
        wrapper = self.bin / "agent-run"
        wrapper.write_text(
            '#!/usr/bin/env python3\nimport os,sys\nassert sys.argv[1] == "native-build"\nos.execvp(sys.argv[2],sys.argv[2:])\n'
        )
        wrapper.chmod(0o755)
        self.addCleanup(patch.stopall)
        patch.object(self.h, "AGENT_RUN", wrapper).start()
        patch.object(self.h, "CACHE_ROOT", self.base / "cache").start()
        patch.dict(
            os.environ,
            {
                "PATH": str(self.bin) + ":" + os.environ["PATH"],
                "FIXTURE_ROOT": str(self.base),
            },
        ).start()

    def test_sync_owned_removals_runtime_preservation_and_aba(self):
        state = self.base / "state"
        source = state / "source"
        self.h.sync_source(self.context, source, state / "manifest.json")
        path = source / "project/src/main.rs"
        first = path.stat().st_mtime_ns
        self.h.sync_source(self.context, source, state / "manifest.json")
        self.assertEqual(path.stat().st_mtime_ns, first)
        (source / "project/runtime/cache").mkdir(parents=True)
        (source / "project/runtime/cache/save").write_text("owned by app")
        incoming = self.context / "project/src/main.rs"
        for text in ("B", "A"):
            incoming.write_text(text)
            os.utime(incoming, (1, 1))
            self.h.sync_source(self.context, source, state / "manifest.json")
            self.assertEqual(path.read_text(), text)
            self.assertGreater(path.stat().st_mtime_ns, first)
            first = path.stat().st_mtime_ns
        incoming.unlink()
        self.h.sync_source(self.context, source, state / "manifest.json")
        self.assertFalse(path.exists())
        self.assertEqual(
            (source / "project/runtime/cache/save").read_text(), "owned by app"
        )

    def test_local_original_root_build_and_runtime_status(self):
        root = self.context / "project"
        status = self.h.execute(
            self.context,
            "key",
            "project",
            ["build"],
            "local",
            ["17"],
            "fixture",
            environment={"SETTING": "native"},
            root=root,
        )
        self.assertEqual(status, 17)
        report = json.loads((root / "target/observed.json").read_text())
        self.assertEqual(report["cwd"], str(root))
        self.assertEqual(report["setting"], "native")
        with patch.dict(os.environ, {"BUILD_STATUS": "9"}):
            self.assertEqual(
                self.h.execute(
                    self.context, "key", "project", ["build"], "local", root=root
                ),
                9,
            )

    def test_worker_cache_isolation_and_runtime_status(self):
        archive = self.base / "snapshot.tar.gz"
        self.h.pack_directory(self.context, archive)
        for key in ("one", "two"):
            result = self.h.worker(
                archive,
                key,
                "project",
                ["build"],
                ["13"],
                "fixture",
                True,
                {"SETTING": key},
                monitor=False,
            )
            self.assertEqual(result, 13)
            report = json.loads(
                (self.base / f"cache/{key}/target/observed.json").read_text()
            )
            self.assertEqual(
                report["cwd"], str(self.base / f"cache/{key}/source/project")
            )
            self.assertEqual(report["setting"], key)
            self.assertTrue(
                (self.base / f"cache/{key}/target/release/fixture").exists()
            )

    def test_invalid_host_and_missing_local_root_do_not_execute(self):
        with self.assertRaises(ValueError):
            self.h.execute(self.context, "key", "project", ["build"], "other")
        with self.assertRaises(ValueError):
            self.h.execute(self.context, "key", "project", ["build"], "local")
        with patch.object(
            self.h, "windows_profile", side_effect=OSError("host unavailable")
        ):
            with self.assertRaisesRegex(OSError, "host unavailable"):
                self.h.execute(self.context, "key", "project", ["build"], "desktop")
        self.assertFalse((self.context / "project/target").exists())

    def cleanup_fixture(self, action):
        marker = self.base / "child.pid"
        code = (
            "import sys; from pathlib import Path; sys.path.insert(0,sys.argv[1]); "
            "import native_build_hosts as h; "
            'sys.exit(h.run_owned([sys.executable,"-c",sys.argv[3]],Path(sys.argv[2]),dict(__import__("os").environ),monitor=True))'
        )
        child = f"import os,signal,time; from pathlib import Path; signal.signal(signal.SIGTERM,signal.SIG_IGN); Path({str(marker)!r}).write_text(str(os.getpid())); time.sleep(60)"
        process = subprocess.Popen(
            [sys.executable, "-c", code, str(SCRIPTS), str(self.base), child],
            stdin=subprocess.PIPE,
        )
        try:
            deadline = time.monotonic() + 5
            while not marker.exists():
                self.assertLess(time.monotonic(), deadline)
                time.sleep(0.02)
            pid = int(marker.read_text())
            if action == "eof":
                process.stdin.close()
            else:
                process.send_signal(signal.SIGTERM)
            self.assertNotEqual(process.wait(timeout=8), 0)
            with self.assertRaises(ProcessLookupError):
                os.kill(pid, 0)
        finally:
            if process.poll() is None:
                process.kill()
                process.wait()
            if not process.stdin.closed:
                process.stdin.close()

    def test_eof_kills_owned_term_resistant_process(self):
        self.cleanup_fixture("eof")

    def test_signal_kills_owned_term_resistant_process(self):
        self.cleanup_fixture("signal")


if __name__ == "__main__":
    unittest.main()
