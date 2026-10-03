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

TRANSPORT = r"""#!/usr/bin/env python3
import base64, importlib, json, os, pathlib, shlex, shutil, sys
base = pathlib.Path(os.environ['FIXTURE_ROOT'])
name = pathlib.Path(sys.argv[0]).name
args = sys.argv[1:]
if os.environ.get('FAIL_STAGE') == name:
    sys.exit(21)
if name == 'systemd-run':
    assert 'MemoryMax=16G' in args and 'CPUQuota=800%' in args
    index = next(i for i, arg in enumerate(args) if arg == 'rustup' or arg.endswith('/fixture'))
    os.execvp(args[index], args[index:])
if name == 'scp':
    def resolve(value):
        if value.startswith('desktop:'):
            return base / 'windows' / value.removeprefix('desktop:C:/Users/Test User/')
        return pathlib.Path(value)
    shutil.copy2(resolve(args[0]), resolve(args[1])); sys.exit(0)
assert name == 'ssh' and args[0] == 'desktop'
command = args[1]
if command.startswith('powershell.exe'):
    script = base64.b64decode(command.split()[-1]).decode('utf-16-le')
    if script == '$env:USERPROFILE':
        print('C:\\Users\\Test User'); sys.exit(0)
    directory = base / 'windows/data/build-host/transfers' / script.split("'")[1].replace('\\', '/').split('/')[-1]
    if script.startswith('New-Item'):
        directory.mkdir(parents=True)
    else:
        shutil.rmtree(directory)
    sys.exit(0)
values = shlex.split(command)
assert values[:7] == ['wsl.exe', '-d', 'OssoBuild', '-u', 'osso-test', '--exec', 'python3'], values
worker, archive, request = values[7:]
def mapped(value):
    return str(base / 'windows' / value.removeprefix('/mnt/c/Users/Test User/'))
sys.path.insert(0, str(pathlib.Path(mapped(worker)).parent))
h = importlib.import_module('native_build_hosts')
h.CACHE_ROOT = base / 'cache'
sys.exit(h.worker(pathlib.Path(mapped(archive)), *json.loads(request)))
"""

RUSTUP = """#!/usr/bin/env python3
import json, os, pathlib, sys
args = sys.argv[1:]
assert args[:2] == ['run', '1.98.1'], args
if args[2] == 'rustc':
    print(os.environ['FIXTURE_ROOT']); sys.exit(0)
assert args[2] == 'cargo' and '--locked' in args and '-j8' in args, args
if os.environ.get('SLEEP_CARGO'):
    import signal, time
    signal.signal(signal.SIGTERM, signal.SIG_IGN)
    pathlib.Path(os.environ['FIXTURE_ROOT'], 'cargo.pid').write_text(str(os.getpid()))
    time.sleep(60)
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
        for name in ("ssh", "scp", "systemd-run"):
            path = self.bin / name
            path.write_text(TRANSPORT)
            path.chmod(0o755)
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

    def test_desktop_full_transport_status_and_no_exports(self):
        result = self.h.execute(
            self.context,
            "desktop-key",
            "project",
            ["build"],
            "desktop",
            ["12"],
            "fixture",
            environment={"SETTING": "remote"},
        )
        self.assertEqual(result, 12)
        report = json.loads(
            (self.base / "cache/desktop-key/target/observed.json").read_text()
        )
        self.assertEqual(report["setting"], "remote")
        self.assertFalse((self.context / "project/target").exists())
        self.assertEqual(
            list((self.base / "windows/data/build-host/transfers").iterdir()), []
        )
        for stage in ("ssh", "scp"):
            with (
                self.subTest(stage=stage),
                patch.dict(os.environ, {"FAIL_STAGE": stage}),
            ):
                with self.assertRaises(subprocess.CalledProcessError):
                    self.h.execute(
                        self.context, "failed-key", "project", ["build"], "desktop"
                    )
            self.assertFalse((self.base / "cache/failed-key").exists())

    def test_desktop_wrapper_signal_closes_lease_and_kills_remote_cargo(self):
        code = (
            "import sys; from pathlib import Path; sys.path.insert(0,sys.argv[1]); "
            "import native_build_hosts as h; "
            'sys.exit(h.execute(Path(sys.argv[2]),"remote","project",["build"],"desktop",environment={"SLEEP_CARGO":"1"}))'
        )
        process = subprocess.Popen(
            [sys.executable, "-c", code, str(SCRIPTS), str(self.context)]
        )
        marker = self.base / "cargo.pid"
        try:
            deadline = time.monotonic() + 8
            while not marker.exists():
                self.assertLess(time.monotonic(), deadline)
                time.sleep(0.02)
            pid = int(marker.read_text())
            process.send_signal(signal.SIGTERM)
            self.assertEqual(process.wait(timeout=8), 143)
            with self.assertRaises(ProcessLookupError):
                os.kill(pid, 0)
            self.assertEqual(
                list((self.base / "windows/data/build-host/transfers").iterdir()), []
            )
        finally:
            if marker.exists():
                try:
                    os.kill(int(marker.read_text()), signal.SIGKILL)
                except ProcessLookupError:
                    pass
            if process.poll() is None:
                process.kill()
            process.wait()

    def test_prune_refuses_runtime_symlink_parent(self):
        state = self.base / "state"
        source = state / "source"
        manifest = state / "manifest.json"
        self.h.sync_source(self.context, source, manifest)
        incoming = self.context / "project/src/main.rs"
        incoming.unlink()
        (source / "project/src/main.rs").unlink()
        (source / "project/src").rmdir()
        runtime = source / "runtime"
        runtime.mkdir()
        (runtime / "main.rs").write_text("runtime-owned")
        (source / "project/src").symlink_to(runtime, target_is_directory=True)
        # Remove incoming directory so only the prune encounters the alias.
        incoming.parent.rmdir()
        with self.assertRaises(ValueError):
            self.h.sync_source(self.context, source, manifest)
        self.assertEqual((runtime / "main.rs").read_text(), "runtime-owned")

    def test_sysroot_query_is_owned_and_bounded(self):
        script = RUSTUP.replace(
            "print(os.environ['FIXTURE_ROOT']); sys.exit(0)",
            "import time, signal; signal.signal(signal.SIGTERM, signal.SIG_IGN); pathlib.Path(os.environ['FIXTURE_ROOT'], 'query.pid').write_text(str(os.getpid())); time.sleep(60)",
        )
        (self.bin / "rustup").write_text(script)
        code = (
            "import sys; from pathlib import Path; sys.path.insert(0,sys.argv[1]); "
            "import native_build_hosts as h; h.AGENT_RUN=Path(sys.argv[2]); "
            'sys.exit(h.execute(Path(sys.argv[3]),"key","project",["build"],"local",["0"],"fixture",root=Path(sys.argv[3])/"project"))'
        )
        process = subprocess.Popen(
            [
                sys.executable,
                "-c",
                code,
                str(SCRIPTS),
                str(self.bin / "agent-run"),
                str(self.context),
            ]
        )
        marker = self.base / "query.pid"
        try:
            deadline = time.monotonic() + 5
            while not marker.exists():
                self.assertLess(time.monotonic(), deadline)
                time.sleep(0.02)
            pid = int(marker.read_text())
            process.send_signal(signal.SIGTERM)
            self.assertEqual(process.wait(timeout=6), 143)
            with self.assertRaises(ProcessLookupError):
                os.kill(pid, 0)
        finally:
            # RED must also clean the unowned query, not leave a sleeper behind.
            if marker.exists():
                try:
                    os.kill(int(marker.read_text()), signal.SIGKILL)
                except ProcessLookupError:
                    pass
            if process.poll() is None:
                process.kill()
            process.wait()

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
