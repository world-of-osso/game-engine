"""Bounded native transport process fixtures; no real cargo/host operations."""

import importlib
import json
import os
import signal
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path
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
    import signal, subprocess, time
    while args and args[0] == '-o':
        del args[:2]
    with (base / 'uploads.log').open('a') as log:
        log.write(json.dumps(args) + '\n')
    mode = os.environ.get('UPLOAD_MODE', '')
    attempts = len((base / 'uploads.log').read_text().splitlines())
    if mode in ('stall', 'stall-once') and (mode == 'stall' or attempts == 1):
        child = subprocess.Popen([sys.executable, '-c', 'import time; time.sleep(60)'])
        def stop(number, frame):
            # Group TERM must reach the child; the parent cannot terminate it.
            child.wait()
        signal.signal(signal.SIGTERM, stop)
        (base / ('upload-%s.json' % attempts)).write_text(json.dumps([os.getpid(), child.pid]))
        print('upload stalled', file=sys.stderr, flush=True)
        time.sleep(60)
    if mode == 'transient' or (mode == 'transient-once' and attempts == 1):
        print('Connection reset by peer', file=sys.stderr); sys.exit(17)
    if mode == 'permanent':
        print(os.environ.get('UPLOAD_ERROR', 'Permission denied (publickey)\nscp: Connection closed'), file=sys.stderr); sys.exit(19)
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
original_exists = pathlib.Path.exists
pathlib.Path.exists = lambda path: True if str(path).startswith('/run/user/') and path.name == 'bus' else original_exists(path)
sys.exit(h.worker(pathlib.Path(mapped(archive)), *json.loads(request)))
"""

RUSTUP = """#!/usr/bin/env python3
import json, os, pathlib, sys
args = sys.argv[1:]
if pathlib.Path(sys.argv[0]).name in ('cargo', 'rustc'):
    args = ['run', '1.98.1', pathlib.Path(sys.argv[0]).name, *args]
assert args[:2] == ['run', '1.98.1'], args
if args[2] == 'rustc':
    print(os.environ['FIXTURE_ROOT']); sys.exit(0)
assert args[2] == 'cargo' and '--locked' in args and '-j8' in args, args
with pathlib.Path(os.environ['FIXTURE_ROOT'], 'cargo.log').open('a') as log:
    log.write(json.dumps(args) + '\\n')
if '--' in args and any(arg in ('--locked', '-j8') for arg in args[args.index('--') + 1:]):
    print('test harness received Cargo-only flags', file=sys.stderr); sys.exit(31)
if os.environ.get('SLEEP_CARGO'):
    import signal, time
    signal.signal(signal.SIGTERM, signal.SIG_IGN)
    pathlib.Path(os.environ['FIXTURE_ROOT'], 'cargo.pid').write_text(str(os.getpid()))
    time.sleep(60)
root = pathlib.Path.cwd()
target = pathlib.Path(os.environ['CARGO_TARGET_DIR'])
target.mkdir(parents=True, exist_ok=True)
(target / 'observed.json').write_text(json.dumps({'cwd': str(root), 'args': args, 'setting': os.environ.get('SETTING'), 'path': os.environ['PATH'], 'bus': os.environ.get('DBUS_SESSION_BUS_ADDRESS')}))
profile = 'release' if '--release' in args else 'debug'
(target / profile).mkdir(exist_ok=True)
app = target / profile / 'fixture'
app.write_text("#!/usr/bin/env python3\\nimport json, os, pathlib, signal, sys, time\\nbase = pathlib.Path(os.environ['FIXTURE_ROOT'])\\n(base / 'runtime.json').write_text(json.dumps({'cwd': os.getcwd(), 'path': os.environ['PATH'], 'loader': os.environ.get('LD_LIBRARY_PATH'), 'bus': os.environ.get('DBUS_SESSION_BUS_ADDRESS')}))\\nif os.environ.get('SLEEP_RUNTIME'):\\n    signal.signal(signal.SIGTERM, signal.SIG_IGN)\\n    (base / 'runtime.pid').write_text(str(os.getpid()))\\n    time.sleep(60)\\nsys.exit(int(sys.argv[1]))\\n")
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
        original_exists = Path.exists
        patch.object(
            Path,
            "exists",
            lambda path: (
                True
                if str(path).startswith("/run/user/") and path.name == "bus"
                else original_exists(path)
            ),
        ).start()
        for name in ("ssh", "scp", "systemd-run"):
            path = self.bin / name
            path.write_text(TRANSPORT)
            path.chmod(0o755)
        self.home = self.base / "home"
        self.cargo = self.home / ".cargo/bin"
        self.cargo.mkdir(parents=True)
        (self.cargo / "rustup").write_text(RUSTUP)
        (self.cargo / "rustup").chmod(0o755)
        for name in ("cargo", "rustc"):
            path = self.bin / name
            path.write_text(RUSTUP)
            path.chmod(0o755)
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
                "HOME": str(self.home),
            },
        ).start()

    def upload_fixture(self, mode, cancel=None):
        old = self.base / "cache/old/source/project/state"
        old.parent.mkdir(parents=True, exist_ok=True)
        old.write_bytes(b"previous project state")
        code = (
            "import sys; from pathlib import Path; sys.path.insert(0,sys.argv[1]); "
            "import native_build_hosts as h; h.UPLOAD_TIMEOUT=.3; h.UPLOAD_BACKOFF=.02; "
            'sys.exit(h.execute(Path(sys.argv[2]),"upload","project",["build"],"desktop",build=False))'
        )
        env = dict(os.environ, UPLOAD_MODE=mode)
        process = subprocess.Popen(
            [sys.executable, "-c", code, str(SCRIPTS), str(self.context)],
            env=env,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        try:
            if cancel is not None:
                deadline = time.monotonic() + 4
                while not (self.base / "upload-1.json").exists():
                    self.assertLess(time.monotonic(), deadline)
                    time.sleep(0.02)
                process.send_signal(cancel)
            try:
                _, stderr = process.communicate(timeout=12)
            except subprocess.TimeoutExpired:
                self.fail("native upload did not finish within bounded wait")
            for marker in self.base.glob("upload-*.json"):
                for pid in json.loads(marker.read_text()):
                    with self.assertRaises(ProcessLookupError):
                        os.kill(pid, 0)
            self.assertEqual(old.read_bytes(), b"previous project state")
            self.assertFalse((self.context / "project/target").exists())
            self.assertFalse((self.base / "cargo.log").exists())
            self.assertEqual(
                list((self.base / "windows/data/build-host/transfers").iterdir()), []
            )
            return (
                process.returncode,
                stderr.decode(),
                [
                    json.loads(line)
                    for line in (self.base / "uploads.log").read_text().splitlines()
                ],
            )
        finally:
            # RED must not leave either the unowned SCP or its descendants running.
            for marker in self.base.glob("upload-*.json"):
                parent, child = json.loads(marker.read_text())
                try:
                    os.kill(child, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                try:
                    os.kill(parent, signal.SIGTERM)
                    time.sleep(0.05)
                    os.kill(parent, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                try:
                    os.kill(child, signal.SIGKILL)
                except ProcessLookupError:
                    pass
            if process.poll() is None:
                process.kill()
            process.communicate()

    def test_upload_timeout_retries_three_and_reaps_descendants(self):
        status, error, attempts = self.upload_fixture("stall")
        self.assertNotEqual(status, 0)
        self.assertEqual(len(attempts), 3)
        self.assertIn("source.tar.gz", error)
        self.assertIn("desktop:", error)
        self.assertIn("timed out", error)
        self.assertIn("upload stalled", error)
        self.assertFalse((self.base / "cache/upload").exists())

    def test_upload_cancellation_reaps_owned_descendants(self):
        for number in (signal.SIGINT, signal.SIGTERM):
            with self.subTest(signal=number):
                status, _, attempts = self.upload_fixture("stall", number)
                self.assertEqual(status, 128 + number)
                self.assertEqual(len(attempts), 1)
                (self.base / "uploads.log").unlink()
                (self.base / "upload-1.json").unlink()
                (self.base / "cache/old/source/project/state").unlink()
                # Reuse the existing cache directory in the next signal case.

    def test_upload_transient_once_transfers_successful_bytes(self):
        status, _, attempts = self.upload_fixture("transient-once")
        self.assertEqual(status, 0)
        self.assertEqual(len(attempts), 4)
        self.assertEqual(
            (self.base / "cache/upload/source/project/src/main.rs").read_bytes(), b"A"
        )

    def test_upload_timeout_once_transfers_successful_bytes(self):
        status, _, attempts = self.upload_fixture("stall-once")
        self.assertEqual(status, 0)
        self.assertEqual(len(attempts), 4)
        self.assertEqual(
            (self.base / "cache/upload/source/project/src/main.rs").read_bytes(), b"A"
        )

    def test_upload_permanent_failure_does_not_retry(self):
        for message in (
            "Permission denied (publickey)\nscp: Connection closed",
            "scp: dest: No such file or directory\nscp: Connection closed",
            "scp: dest: Permission denied\nscp: Connection closed",
        ):
            with (
                self.subTest(error=message),
                patch.dict(os.environ, {"UPLOAD_ERROR": message}),
            ):
                status, error, attempts = self.upload_fixture("permanent")
                self.assertNotEqual(status, 0)
                self.assertEqual(len(attempts), 1)
                self.assertIn(message.splitlines()[0], error)
                self.assertIn("19", error)
                self.assertFalse((self.base / "cache/upload").exists())
                (self.base / "uploads.log").unlink()

    def test_upload_transient_failures_stop_after_three(self):
        status, error, attempts = self.upload_fixture("transient")
        self.assertNotEqual(status, 0)
        self.assertEqual(len(attempts), 3)
        self.assertIn("Connection reset by peer", error)

    def test_no_build_runs_existing_profiles_and_syncs_desktop_without_cargo(self):
        for host in ("local", "desktop"):
            for release in (False, True):
                with self.subTest(host=host, release=release):
                    target = (
                        self.context / "project/target"
                        if host == "local"
                        else self.base / "cache/no-build/target"
                    )
                    profile = "release" if release else "debug"
                    app = target / profile / "fixture"
                    app.parent.mkdir(parents=True, exist_ok=True)
                    app.write_text(
                        "#!/usr/bin/env python3\n"
                        "import json, os, pathlib, sys\n"
                        "pathlib.Path(os.environ['FIXTURE_ROOT'], 'runtime.json').write_text("
                        "json.dumps(dict(args=sys.argv[1:], cwd=os.getcwd(), loader=os.environ['LD_LIBRARY_PATH'])))\n"
                        "sys.exit(23)\n"
                    )
                    app.chmod(0o755)
                    asset = self.context / "project/addon.lua"
                    asset.write_text(profile)
                    status = self.h.execute(
                        self.context,
                        "no-build",
                        "project",
                        ["build"],
                        host,
                        ["two words"],
                        "fixture",
                        release=release,
                        root=self.context / "project",
                        build=False,
                    )
                    self.assertEqual(status, 23)
                    report = json.loads((self.base / "runtime.json").read_text())
                    self.assertEqual(report["args"], ["two words"])
                    self.assertIn(
                        str(target / profile / "deps"), report["loader"].split(":")
                    )
                    self.assertIn(str(self.base / "lib"), report["loader"].split(":"))
                    self.assertFalse((self.base / "cargo.log").exists())
                    if host == "desktop":
                        self.assertEqual(
                            (
                                self.base / "cache/no-build/source/project/addon.lua"
                            ).read_text(),
                            profile,
                        )
                        self.assertEqual(
                            list(
                                (
                                    self.base / "windows/data/build-host/transfers"
                                ).iterdir()
                            ),
                            [],
                        )

    def test_worker_cli_invalid_key_reports_failure(self):
        request = ["invalid/key", "project", ["build"], None, None, False, {}]
        result = subprocess.run(
            [
                sys.executable,
                str(SCRIPTS / "native_build_hosts.py"),
                "unused.tar.gz",
                json.dumps(request),
            ],
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(result.returncode, 1)
        self.assertIn("native worker failed: invalid checkout cache key", result.stderr)
        self.assertNotIn("Traceback", result.stderr)
        self.assertFalse((self.base / "cargo.log").exists())

    def test_worker_cli_unexpected_request_keeps_traceback(self):
        result = subprocess.run(
            [
                sys.executable,
                str(SCRIPTS / "native_build_hosts.py"),
                "unused.tar.gz",
                "[]",
            ],
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(result.returncode, 1)
        self.assertIn("Traceback", result.stderr)
        self.assertIn("TypeError", result.stderr)
        self.assertFalse((self.base / "cargo.log").exists())

    def test_no_build_missing_binary_fails_without_cargo(self):
        for host in ("local", "desktop"):
            with self.subTest(host=host):
                arguments = {
                    "runtime_args": [],
                    "binary": "fixture",
                    "root": self.context / "project",
                    "build": False,
                }
                if host == "local":
                    with self.assertRaisesRegex(FileNotFoundError, "fixture"):
                        self.h.execute(
                            self.context,
                            "missing",
                            "project",
                            ["build"],
                            host,
                            **arguments,
                        )
                else:
                    self.assertNotEqual(
                        self.h.execute(
                            self.context,
                            "missing",
                            "project",
                            ["build"],
                            host,
                            **arguments,
                        ),
                        0,
                    )
                self.assertFalse((self.base / "cargo.log").exists())
                self.assertFalse((self.base / "runtime.json").exists())

    def test_test_harness_receives_only_its_arguments(self):
        status = self.h.execute(
            self.context,
            "test-args",
            "project",
            ["test", "--test", "integration", "filter", "--", "--nocapture"],
            "local",
            root=self.context / "project",
        )
        self.assertEqual(status, 0)
        arguments = json.loads(
            (self.context / "project/target/observed.json").read_text()
        )["args"]
        self.assertEqual(arguments[arguments.index("--") + 1 :], ["--nocapture"])

    def test_runtime_cwd_environment_local_and_desktop(self):
        runtime = self.base / "prepared-assets"
        runtime.mkdir()
        for host in ("local", "desktop"):
            with self.subTest(host=host):
                status = self.h.execute(
                    self.context,
                    "cwd",
                    "project",
                    ["build"],
                    host,
                    ["0"],
                    "fixture",
                    root=self.context / "project",
                    runtime_cwd=runtime,
                )
                self.assertEqual(status, 0)
                report = json.loads((self.base / "runtime.json").read_text())
                self.assertEqual(report["cwd"], str(runtime))
                self.assertEqual(
                    report["path"].split(":")[0], str(Path.home() / ".cargo/bin")
                )
                loader = report["loader"].split(":")
                self.assertIn(
                    str(self.base / "lib/rustlib/x86_64-unknown-linux-gnu/lib"), loader
                )
                target = (
                    self.context / "project/target"
                    if host == "local"
                    else self.base / "cache/cwd/target"
                )
                build = json.loads((target / "observed.json").read_text())
                self.assertEqual(
                    build["cwd"],
                    str(
                        self.context / "project"
                        if host == "local"
                        else self.base / "cache/cwd/source/project"
                    ),
                )
                self.assertEqual(
                    build["path"].split(":")[0], str(Path.home() / ".cargo/bin")
                )
                if host == "desktop":
                    self.assertEqual(
                        report["bus"], f"unix:path=/run/user/{os.getuid()}/bus"
                    )
                    self.assertEqual(build["bus"], report["bus"])

    def test_missing_desktop_bus_fails_before_build(self):
        archive = self.base / "snapshot.tar.gz"
        self.h.pack_directory(self.context, archive)
        original_exists = Path.exists
        with (
            patch.object(
                Path,
                "exists",
                lambda path: (
                    False
                    if str(path).startswith("/run/user/") and path.name == "bus"
                    else original_exists(path)
                ),
            ),
            self.assertRaisesRegex(RuntimeError, "bus"),
        ):
            self.h.worker(
                archive,
                "bus",
                "project",
                ["build"],
                None,
                None,
                False,
                {},
                monitor=False,
            )
        self.assertFalse((self.base / "cache/bus/target/observed.json").exists())

    def test_second_build_during_runtime_and_stopped_heartbeat_cleanup(self):
        archive = self.base / "snapshot.tar.gz"
        self.h.pack_directory(self.context, archive)
        code = (
            "import sys,json,pathlib; sys.path.insert(0,sys.argv[1]); "
            "import native_build_hosts as h; h.CACHE_ROOT=pathlib.Path(sys.argv[2]); "
            "original=pathlib.Path.exists; "
            "pathlib.Path.exists=lambda p: True if str(p).startswith('/run/user/') and p.name=='bus' else original(p); "
            "sys.exit(h.worker(pathlib.Path(sys.argv[3]),*json.loads(sys.argv[4])))"
        )

        def start(runtime):
            request = [
                "parallel",
                "project",
                ["build"],
                ["0"] if runtime else None,
                "fixture" if runtime else None,
                False,
                {"SLEEP_RUNTIME": "1"} if runtime else {},
            ]
            return subprocess.Popen(
                [
                    sys.executable,
                    "-c",
                    code,
                    str(SCRIPTS),
                    str(self.base / "cache"),
                    str(archive),
                    json.dumps(request),
                ],
                stdin=subprocess.PIPE,
            )

        first = start(True)
        second = None
        marker = self.base / "runtime.pid"
        try:
            deadline = time.monotonic() + 6
            while not marker.exists():
                self.assertLess(time.monotonic(), deadline)
                first.stdin.write(b".\n")
                first.stdin.flush()
                time.sleep(0.05)
            pid = int(marker.read_text())
            second = start(False)
            second.stdin.write(b".\n")
            second.stdin.flush()
            self.assertEqual(
                second.wait(timeout=5), 0, "second build blocked by active runtime"
            )
            self.assertIsNone(first.poll())
            os.kill(pid, 0)
            # Keep stdin open but stop heartbeats: exercise lease expiry, not EOF.
            self.assertEqual(first.wait(timeout=19), 129)
            with self.assertRaises(ProcessLookupError):
                os.kill(pid, 0)
        finally:
            for process in (first, second):
                if process is not None:
                    if process.poll() is None:
                        process.send_signal(signal.SIGTERM)
                        process.wait(timeout=6)
                    process.stdin.close()
            if marker.exists():
                try:
                    os.kill(int(marker.read_text()), signal.SIGKILL)
                except ProcessLookupError:
                    pass

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

    def test_local_runs_without_rustup(self):
        (self.cargo / "rustup").unlink()
        root = self.context / "project"
        status = self.h.execute(
            self.context,
            "key",
            "project",
            ["build"],
            "local",
            ["17"],
            "fixture",
            root=root,
        )
        self.assertEqual(status, 17)
        self.assertEqual(
            json.loads((self.base / "runtime.json").read_text())["cwd"], str(root)
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
        self.assertEqual(
            json.loads((self.base / "runtime.json").read_text())["cwd"], str(root)
        )
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
            self.assertEqual(
                json.loads((self.base / "runtime.json").read_text())["cwd"],
                str(self.base / f"cache/{key}/source/project"),
            )
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
        with (
            patch.object(
                self.h, "windows_profile", side_effect=OSError("host unavailable")
            ),
            self.assertRaisesRegex(OSError, "host unavailable"),
        ):
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
                self.assertRaises(subprocess.CalledProcessError),
            ):
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
        (self.bin / "rustc").write_text(script)
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
            self.assertFalse((self.base / "runtime.json").exists())
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
