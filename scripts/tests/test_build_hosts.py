"""Transport boundary tests: executable fakes never invoke Docker/SSH services."""

import importlib.util
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile
import time
import unittest
from unittest.mock import patch

SCRIPT = Path(__file__).resolve().parents[1] / "build_hosts.py"

FAKE = r"""#!/usr/bin/env python3
import base64, importlib.util, io, json, os, pathlib, shutil, subprocess, sys, tarfile
base = pathlib.Path(os.environ['HOST_TEST_BASE'])
args = sys.argv[1:]
name = pathlib.Path(sys.argv[0]).name
if name == 'systemd-run':
    # Run only the explicitly installed fake Docker, not anything from the host.
    assert '--slice=agents-build_host.slice' in args, args
    args = args[args.index('--') + 1:]
    assert args[:3] == ['ionice', '-c', '3'], args
    sys.exit(subprocess.call([str(base / 'bin/docker'), *args[4:]]))
if name == 'docker':
    assert args[:3] == ['buildx', 'build', '--builder'], args
    assert args[3] == 'game-engine', args
    assert args[args.index('--platform') + 1] == 'linux/amd64', args
    context = pathlib.Path(args[-1])
    assert pathlib.Path(args[args.index('--file') + 1]) == context / 'Dockerfile'
    opts = [args[i + 1] for i, a in enumerate(args) if a == '--build-arg']
    assert 'JOBS=8' in opts and 'TARGET_CACHE=godot-target-' + os.environ['EXPECTED_KEY'] in opts
    if os.environ.get('FAIL_STAGE') == 'docker':
        sys.exit(7)
    if os.environ.get('BARRIER'):
        import time
        (base / os.environ['BARRIER']).touch()
        deadline = time.monotonic() + 10
        while not (base / 'release').exists():
            if time.monotonic() > deadline:
                sys.exit(10)
            time.sleep(0.01)
    output = pathlib.Path(args[args.index('--output') + 1].split('dest=', 1)[1])
    output.mkdir(parents=True, exist_ok=True)
    files = {str(p.relative_to(context)): p.read_text() for p in context.rglob('*') if p.is_file()}
    artifact = {'context': str(context), 'files': files, 'mtime': (context / 'src/lib.rs').stat().st_mtime_ns,
                'target': args[args.index('--target') + 1], 'options': opts}
    (output / 'artifact.json').write_text(json.dumps(artifact))
    (output / 'client').write_text('executable artifact')
    (output / 'client').chmod(0o755)
    sys.exit(0)
if name == 'scp':
    if os.environ.get('FAIL_STAGE') == 'scp':
        sys.exit(9)
    src, dst = args[-2:]
    def resolve(value):
        if value.startswith('desktop:'):
            return base / 'windows' / value.removeprefix('desktop:').replace('C:/Users/Test User/', '')
        return pathlib.Path(value)
    shutil.copy2(resolve(src), resolve(dst))
    if os.environ.get('FAIL_STAGE') == 'unsafe-output' and src.endswith('output.tar.gz'):
        with tarfile.open(resolve(dst), 'w:gz') as archive:
            entry = tarfile.TarInfo('../escaped')
            entry.size = 4
            archive.addfile(entry, io.BytesIO(b'evil'))
    sys.exit(0)
assert name == 'ssh' and args[0] == 'desktop', args
if os.environ.get('FAIL_STAGE') == 'ssh':
    sys.exit(8)
command = args[1]
if command.startswith('powershell.exe'):
    ps = base64.b64decode(command.split()[-1]).decode('utf-16-le')
    if ps == '$env:USERPROFILE':
        sys.stdout.buffer.write('C:\\Users\\Test User\r\n'.encode('utf-16-le'))
    else:
        transfer = base / 'windows/data/build-host/transfers' / ps.split("'")[1].replace('\\', '/').split('/')[-1]
        if ps.startswith('New-Item'):
            transfer.mkdir(parents=True)
        else:
            shutil.rmtree(transfer)
    sys.exit(0)
assert command.startswith('wsl.exe -d OssoBuild -u root --exec python3 '), command
# Windows argv decoding for quoted paths; these fixtures contain no quotes/backslashes.
import shlex
values = shlex.split(command)
def mapped(value):
    return str(base / 'windows' / value.removeprefix('/mnt/c/Users/Test User/'))
worker, archive, output, key, target, build_args = values[7:]
spec = importlib.util.spec_from_file_location('remote_worker', mapped(worker))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
module.CACHE_ROOT = base / 'ext4'
module.worker(pathlib.Path(mapped(archive)), pathlib.Path(mapped(output)), key, target, json.loads(build_args))
"""


class BuildHostTests(unittest.TestCase):
    def setUp(self):
        self.assertTrue(SCRIPT.is_file(), "build transport module missing")
        spec = importlib.util.spec_from_file_location("build_hosts", SCRIPT)
        self.module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.module)
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name)
        self.context = self.base / "context"
        (self.context / "src").mkdir(parents=True)
        (self.context / "Dockerfile").write_text("FROM scratch\n")
        self.source = self.context / "src/lib.rs"
        self.source.write_text("original source")
        self.mtime = 1700000000123456789
        os.utime(self.source, ns=(self.mtime, self.mtime))
        self.output = self.base / "output"
        binary = self.base / "bin"
        binary.mkdir()
        for name in ("ssh", "scp", "docker", "systemd-run"):
            path = binary / name
            path.write_text(FAKE)
            path.chmod(0o755)
        env = {
            "PATH": str(binary) + os.pathsep + os.environ["PATH"],
            "HOST_TEST_BASE": str(self.base),
            "EXPECTED_KEY": "checkout-a",
        }
        self.environment = patch.dict(os.environ, env)
        self.environment.start()
        self.addCleanup(self.environment.stop)

    def execute(self, host="desktop", key="checkout-a"):
        os.environ["EXPECTED_KEY"] = key
        self.module.execute(
            self.context,
            self.output,
            key,
            "fixture",
            ["--build-arg", "FIXTURE=native_input_fixture"],
            host,
        )
        return json.loads((self.output / "artifact.json").read_text())

    def assert_clean_transfers(self):
        transfers = self.base / "windows/data/build-host/transfers"
        self.assertFalse(transfers.exists() and list(transfers.iterdir()))

    def test_local_artifacts_and_build_contract(self):
        result = self.execute("local")
        self.assertEqual(
            result["files"],
            {"Dockerfile": "FROM scratch\n", "src/lib.rs": "original source"},
        )
        self.assertEqual(result["target"], "fixture")
        self.assertIn("FIXTURE=native_input_fixture", result["options"])
        self.assertEqual(result["mtime"], self.mtime)

    def test_desktop_roundtrip_stable_context_mtime_and_stale_removal(self):
        first = self.execute()
        self.assertEqual(first["mtime"], self.mtime)
        self.assertEqual((self.output / "client").stat().st_mode & 0o777, 0o755)
        self.assert_clean_transfers()
        (self.context / "removed.rs").write_text("old")
        self.execute()
        (self.context / "removed.rs").unlink()
        self.source.write_text("updated source")
        result = self.execute()
        self.assertEqual(result["context"], first["context"])
        self.assertEqual(result["files"]["src/lib.rs"], "updated source")
        self.assertNotIn("removed.rs", result["files"])
        other = self.execute(key="checkout-b")
        self.assertNotEqual(other["context"], first["context"])
        self.assert_clean_transfers()

    def test_transport_and_build_failures_do_not_fallback(self):
        for stage in ("ssh", "scp", "docker"):
            with (
                self.subTest(stage=stage),
                patch.dict(os.environ, {"FAIL_STAGE": stage}),
            ):
                with self.assertRaises(subprocess.CalledProcessError):
                    self.execute()
                self.assertFalse(self.output.exists())
            self.assert_clean_transfers()

    def test_local_build_failure_is_explicit(self):
        with patch.dict(os.environ, {"FAIL_STAGE": "docker"}):
            with self.assertRaises(subprocess.CalledProcessError) as error:
                self.execute("local")
        self.assertEqual(error.exception.returncode, 7)

    def test_reject_unsafe_download_without_writing_outside_output(self):
        with patch.dict(os.environ, {"FAIL_STAGE": "unsafe-output"}):
            with self.assertRaises(ValueError):
                self.execute()
        self.assertFalse((self.base / "escaped").exists())
        self.assert_clean_transfers()

    def test_worker_rejects_unsafe_source_members(self):
        self.module.CACHE_ROOT = self.base / "ext4"
        for kind in ("traversal", "symlink", "hardlink", "absolute"):
            with self.subTest(kind=kind):
                archive = self.base / "source.tar.gz"
                with tarfile.open(archive, "w:gz") as bundle:
                    member = tarfile.TarInfo(
                        "../escaped" if kind == "traversal" else "/escaped" if kind == "absolute" else "link"
                    )
                    if kind in ("symlink", "hardlink"):
                        member.type = tarfile.SYMTYPE if kind == "symlink" else tarfile.LNKTYPE
                        member.linkname = "../escaped"
                    bundle.addfile(member, io.BytesIO())
                with self.assertRaises(ValueError):
                    self.module.worker(
                        archive,
                        self.base / "result.tar.gz",
                        "checkout-a",
                        "fixture",
                        [],
                    )
                self.assertFalse((self.base / "escaped").exists())

    def test_worker_serializes_same_checkout_across_processes(self):
        archive = self.base / "source.tar.gz"
        self.module.pack_directory(self.context, archive)
        script = (
            "import sys; from pathlib import Path; sys.path.insert(0, sys.argv[1]); "
            "import build_hosts as h; h.CACHE_ROOT=Path(sys.argv[2]); "
            "h.worker(Path(sys.argv[3]), Path(sys.argv[4]), 'checkout-a', 'fixture', [])"
        )
        children = []
        try:
            for name in ("first", "second"):
                args = [
                    sys.executable,
                    "-c",
                    script,
                    str(SCRIPT.parent),
                    str(self.base / "ext4"),
                    str(archive),
                    str(self.base / f"{name}.tar.gz"),
                ]
                child = subprocess.Popen(args, env={**os.environ, "BARRIER": name})
                children.append(child)
                if name == "first":
                    deadline = time.monotonic() + 5
                    while (
                        not (self.base / name).exists()
                        and child.poll() is None
                        and time.monotonic() < deadline
                    ):
                        time.sleep(0.01)
                    self.assertTrue((self.base / name).exists(), "first build failed to enter Docker")
            time.sleep(0.2)
            self.assertIsNone(children[1].poll())
            self.assertFalse((self.base / "second").exists(), "second build overlapped first")
        finally:
            (self.base / "release").touch()
            for child in children:
                self.assertEqual(child.wait(timeout=10), 0)
        for name in ("first", "second"):
            self.module.extract_directory(self.base / f"{name}.tar.gz", self.base / f"{name}-result")
            artifact = json.loads((self.base / f"{name}-result/artifact.json").read_text())
            self.assertEqual(artifact["files"]["src/lib.rs"], "original source")

    def test_invalid_host_and_key_fail_before_transport(self):
        for host, key in (("depot", "checkout-a"), ("desktop", "../escape")):
            with self.subTest(host=host, key=key), self.assertRaises(ValueError):
                self.execute(host, key)


if __name__ == "__main__":
    unittest.main()
