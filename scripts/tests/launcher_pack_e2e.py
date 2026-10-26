#!/usr/bin/env python3
"""Run real launcher cold, delta and interrupted installs against a local fixture."""
import hashlib
import json
import os
import subprocess
import sys
import tempfile
import threading
import time
from collections import Counter
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import launcher_packs

AGENT_RUN = '/home/osso/Projects/world-of-osso/game-engine/scripts/agent/agent-run'


class FixtureServer(ThreadingHTTPServer):
    def __init__(self, address, source, output):
        super().__init__(address, Handler)
        self.source = source
        self.output = output
        self.legacy = False
        self.requests = Counter()
        self.lock = threading.Lock()
        self.blocked = None
        self.entered = threading.Event()
        self.release = threading.Event()


class Handler(BaseHTTPRequestHandler):
    def log_message(self, *args):
        pass

    def do_GET(self):
        path = self.path.lstrip('/')
        with self.server.lock:
            self.server.requests[path] += 1
        time.sleep(0.01)
        if path == 'launcher.json' or (path == 'manifest-v2.json' and self.server.legacy):
            self.send_error(404)
            return
        if path == self.server.blocked:
            self.server.entered.set()
            self.server.release.wait(15)
        base = self.server.output if path == 'manifest-v2.json' or path.startswith('packs/') else self.server.source
        target = base / path
        if not target.is_file():
            self.send_error(404)
            return
        data = target.read_bytes()
        header = self.headers.get('Range')
        if header:
            start, end = map(int, header.removeprefix('bytes=').split('-'))
            body = data[start:end + 1]
            self.send_response(206)
            self.send_header('Content-Range', f'bytes {start}-{end}/{len(data)}')
        else:
            body = data
            self.send_response(200)
        self.send_header('Content-Length', str(len(body)))
        self.end_headers()
        try:
            self.wfile.write(body)
        except (BrokenPipeError, ConnectionResetError):
            pass


def compare_tree(source, destination, names):
    for name in names:
        assert (destination / name).read_bytes() == (source / name).read_bytes(), name


def run_launcher(binary, server, directory):
    address = f'http://127.0.0.1:{server.server_port}'
    environment = dict(os.environ, GAME_LAUNCHER_BASE_URL=address,
                       GAME_LAUNCHER_MANIFEST_URL=address + '/launcher.json',
                       GAME_LAUNCHER_GAME_DIR=str(directory))
    return subprocess.Popen([AGENT_RUN, 'launcherpackfixture', binary, 'update'],
                            env=environment, stdout=subprocess.PIPE, stderr=subprocess.PIPE)


def finish(process):
    out, err = process.communicate(timeout=60)
    if process.returncode:
        raise AssertionError(f'launcher exit {process.returncode}: {out!r} {err!r}')


def main(binary):
    with tempfile.TemporaryDirectory(prefix='launcher-pack-e2e-') as temporary:
        root = Path(temporary)
        source, output = root / 'source', root / 'output'
        source.mkdir()
        names = [f'data/{index:03}.bin' for index in range(96)]
        for index, name in enumerate(names):
            path = source / name
            path.parent.mkdir(exist_ok=True)
            path.write_bytes(hashlib.shake_256(str(index).encode()).digest(4096))

        def build():
            files = []
            for name in names:
                data = (source / name).read_bytes()
                files.append({'path': name, 'size': len(data), 'sha256': hashlib.sha256(data).hexdigest()})
            manifest = {'version': 'fixture', 'files': files}
            (source / 'manifest.json').write_text(json.dumps(manifest))
            launcher_packs.build_packs(source, output, manifest)

        launcher_packs.TARGET = 64 * 1024
        build()
        server = FixtureServer(('127.0.0.1', 0), source, output)
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        try:
            def measured(label, directory):
                with server.lock:
                    server.requests.clear()
                start = time.monotonic()
                finish(run_launcher(binary, server, directory))
                elapsed = time.monotonic() - start
                compare_tree(source, directory, names)
                with server.lock:
                    counts = dict(server.requests)
                print(json.dumps({'case': label, 'seconds': round(elapsed, 4),
                                  'requests': sum(counts.values()),
                                  'asset_requests': sum(v for k, v in counts.items() if k.startswith(('packs/', 'data/')))}), flush=True)
                return counts

            packed = root / 'packed'
            measured('pack_cold', packed)
            server.legacy = True
            legacy = root / 'legacy'
            measured('legacy_cold', legacy)
            server.legacy = False
            (source / names[0]).write_bytes(b'changed-one')
            (source / names[1]).write_bytes(b'changed-two')
            build()
            measured('pack_delta', packed)
            server.legacy = True
            measured('legacy_delta', legacy)
            server.legacy = False

            fresh = root / 'resume'
            pack_names = sorted(p.name for p in (output / 'packs').iterdir())
            server.blocked = 'packs/' + pack_names[-1]
            process = run_launcher(binary, server, fresh)
            assert server.entered.wait(15), 'blocked transfer did not start'
            deadline = time.monotonic() + 15
            while time.monotonic() < deadline:
                retained = [p.name for p in (fresh / '.pack-staging').glob('*') if p.is_file() and len(p.name) == 64]
                if retained:
                    break
                time.sleep(0.02)
            assert retained, 'no verified pack was retained before interruption'
            subprocess.run(['systemctl', '--user', 'stop', 'agents-launcherpackfixture.slice'], check=True)
            process.communicate(timeout=15)
            server.release.set()
            server.blocked = None
            counts = measured('pack_resume', fresh)
            assert all('packs/' + name not in counts for name in retained), 'verified pack fetched again'
            print(json.dumps({'case': 'interruption', 'retained_packs': len(retained)}), flush=True)
        finally:
            server.release.set()
            server.shutdown()
            thread.join()
            server.server_close()


if __name__ == '__main__':
    main(sys.argv[1])
