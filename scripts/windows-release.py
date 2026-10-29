#!/usr/bin/env python3
"""Build/stage a Windows GNU player release through ssh desktop. Never publishes."""

import argparse
import base64
import hashlib
import importlib.util
import json
import shutil
import subprocess
import tarfile
from pathlib import Path

from player_bundle import copy_data, copy_project, copy_windows_native

DEFAULT_REMOTE = 'C:/Users/adeia/Projects/woo-windows-build'


def run(*arguments):
    subprocess.run(arguments, check=True)


def powershell(host, script):
    encoded = base64.b64encode(script.encode('utf-16-le')).decode()
    run('ssh', host, 'powershell.exe -NoProfile -EncodedCommand ' + encoded)


def upload(host, source, target):
    run('scp', str(source), f'{host}:{target}')


def load_builder():
    spec = importlib.util.spec_from_file_location('engine_build', Path(__file__).with_name('engine-build.py'))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def stage_source(root, work, host, remote):
    builder = load_builder()
    context = work / 'source'
    if context.exists():
        shutil.rmtree(context)
    context.mkdir(parents=True)
    builder.snapshot(root, context)
    # Native Cargo path dependencies expect the engine to be a sibling with this name.
    (context / builder.ROOT_NAME).rename(context / 'game-engine')
    shutil.copytree(root / 'scripts', context / 'game-engine/scripts', ignore=shutil.ignore_patterns('__pycache__'))
    revisions = {}
    for name in ('game-engine', *builder.SIBLINGS):
        repo = root if name == 'game-engine' else builder.sibling_repo(root, name)
        revisions[name] = subprocess.check_output(['git', '-C', str(repo), 'rev-parse', 'HEAD'], text=True).strip()
        if name != 'game-engine' and subprocess.check_output(['git', '-C', str(repo), 'status', '--porcelain']):
            raise RuntimeError(f'Commit sibling changes before release: {repo}')
    (context / 'revisions.json').write_text(json.dumps(revisions, indent=2) + '\n')
    archive = work / 'source.tar.gz'
    with tarfile.open(archive, 'w:gz') as output:
        for path in context.iterdir():
            output.add(path, arcname=path.name)
    powershell(host, f"$ErrorActionPreference='Stop'; New-Item -ItemType Directory -Force '{remote}/source' | Out-Null")
    upload(host, archive, remote + '/source.tar.gz')
    powershell(host, f"$ErrorActionPreference='Stop'; tar -xf '{remote}/source.tar.gz' -C '{remote}/source'; if ($LASTEXITCODE) {{ exit $LASTEXITCODE }}")
    shutil.copy2(context / 'revisions.json', work / 'revisions.json')


def stage_launcher(root, work, host, remote):
    launcher = load_builder().sibling_repo(root, 'game-launcher')
    revision = subprocess.check_output(['git', '-C', str(launcher), 'rev-parse', 'HEAD'], text=True).strip()
    (work / 'launcher-revision.txt').write_text(revision + '\n')
    output = subprocess.check_output(['git', '-C', str(launcher), 'ls-files', '-z'])
    archive = work / 'launcher-source.tar.gz'
    with tarfile.open(archive, 'w:gz') as target:
        for name in output.split(b'\0'):
            if name:
                relative = name.decode()
                target.add(launcher / relative, arcname='game-launcher/' + relative)
    upload(host, archive, remote + '/launcher-source.tar.gz')
    powershell(host, f"tar -xf '{remote}/launcher-source.tar.gz' -C '{remote}/source'; exit $LASTEXITCODE")
    upload(host, root / 'scripts/windows-worker.py', remote + '/source/game-engine/scripts/windows-worker.py')


def worker(host, remote, phase):
    powershell(host,
        f"$ErrorActionPreference='Stop'; $env:PYTHONUTF8='1'; & '{remote}/tools/uv/uv.exe' run --python 3.12.12 "
        f"'{remote}/source/game-engine/scripts/windows-worker.py' --root '{remote}' --phase {phase}; exit $LASTEXITCODE")


def bootstrap(host, remote):
    powershell(host,
        f"$ErrorActionPreference='Stop'; $ProgressPreference='SilentlyContinue'; "
        f"New-Item -ItemType Directory -Force '{remote}/tools' | Out-Null; "
        f"if (!(Test-Path '{remote}/tools/uv/uv.exe')) {{ "
        f"Invoke-WebRequest 'https://github.com/astral-sh/uv/releases/download/0.9.2/uv-x86_64-pc-windows-msvc.zip' -OutFile '{remote}/tools/uv.zip'; "
        f"Expand-Archive '{remote}/tools/uv.zip' '{remote}/tools/uv' }}; "
        f"& '{remote}/tools/uv/uv.exe' python install 3.12.12; if ($LASTEXITCODE) {{ exit $LASTEXITCODE }}; "
        "rustup toolchain install 1.98.1 --profile minimal --target x86_64-pc-windows-gnu; exit $LASTEXITCODE")


def stage_payload(root, work, host, remote):
    bundle = root / 'target/deploy/windows-x86_64'
    if bundle.exists():
        shutil.rmtree(bundle)
    (bundle / 'lib').mkdir(parents=True)
    (bundle / 'runtime').mkdir()
    copy_project(root, bundle, 'windows-x86_64')
    copy_data(root, bundle)
    archive = work / 'player.tar.zst'
    create_archive(bundle, archive)
    powershell(host, f"$ErrorActionPreference='Stop'; if (Test-Path '{remote}/player') {{ Remove-Item -Recurse -Force '{remote}/player' }}")
    stream_archive(host, archive, remote + '/player')


def create_archive(bundle, archive):
    with archive.open('wb') as output:
        compressor = subprocess.Popen(['zstd', '-T4', '-3', '-q'], stdin=subprocess.PIPE, stdout=output)
        producer = subprocess.Popen(['tar', '--dereference', '-cf', '-', '-C', str(bundle), '.'], stdout=compressor.stdin)
        compressor.stdin.close()
        produced = producer.wait()
        compressed = compressor.wait()
    if produced or compressed:
        raise RuntimeError(f'Player archive failed: tar={produced}, zstd={compressed}')


def stream_archive(host, archive, destination):
    powershell(host, f"$ErrorActionPreference='Stop'; New-Item -ItemType Directory -Force '{destination}' | Out-Null; compact.exe /C '{destination}'; exit $LASTEXITCODE")
    # Native tar inherits binary stdin directly. No PowerShell text pipeline and no
    # second 172GB archive copy on Windows. NTFS compression is inherited by new files.
    script = f"& tar.exe -xf - -C '{destination}'; exit $LASTEXITCODE"
    encoded = base64.b64encode(script.encode('utf-16-le')).decode()
    with archive.open('rb') as source:
        subprocess.run(['ssh', host, 'powershell.exe -NoProfile -EncodedCommand ' + encoded], stdin=source, check=True)


def verify_runtime_checksum(artifacts):
    runtime = artifacts / 'godot.exe'
    expected = (artifacts / 'godot.sha512').read_text().strip()
    with runtime.open('rb') as stream:
        actual = hashlib.file_digest(stream, 'sha512').hexdigest()
    if actual != expected:
        raise ValueError(f'Windows godot.exe SHA-512 mismatch: {runtime}')


def fetch_artifacts(work, host, remote):
    artifacts = work / 'artifacts'
    artifacts.mkdir(parents=True, exist_ok=True)
    for name in ('game_engine_godot.dll', 'godot.exe', 'game-engine-windows-x86_64.exe', 'godot.sha512'):
        run('scp', f'{host}:{remote}/artifacts/{name}', str(artifacts / name))
    verify_runtime_checksum(artifacts)
    return artifacts


def finish_bundle(root, work, host, remote):
    bundle = root / 'target/deploy/windows-x86_64'
    copy_windows_native(fetch_artifacts(work, host, remote), bundle)
    native = work / 'native.tar'
    with tarfile.open(native, 'w') as output:
        for name in ('runtime', 'lib', 'game-engine-windows-x86_64'):
            output.add(bundle / name, arcname=name)
    stream_archive(host, native, remote + '/player')
    worker(host, remote, 'import')
    # Import cache must ship in the bundle assembled on Linux, too.
    powershell(host, f"tar -cf '{remote}/import.tar' -C '{remote}/player/godot' .godot; exit $LASTEXITCODE")
    run('scp', f'{host}:{remote}/import.tar', str(work / 'import.tar'))
    with tarfile.open(work / 'import.tar') as archive_input:
        archive_input.extractall(bundle / 'godot', filter='data')
    print(f'Windows player bundle: {bundle}', flush=True)


def assemble(root, work, host, remote):
    stage_payload(root, work, host, remote)
    finish_bundle(root, work, host, remote)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument('--host', default='desktop')
    parser.add_argument('--remote-root', default=DEFAULT_REMOTE)
    parser.add_argument('--phase', choices=('build', 'artifacts', 'assemble', 'all', 'launcher', 'launcher-test', 'launcher-lock'), default='all')
    args = parser.parse_args()
    root = args.root.resolve()
    work = root / 'target/windows-release'
    work.mkdir(parents=True, exist_ok=True)
    if args.phase in ('launcher', 'launcher-test', 'launcher-lock'):
        bootstrap(args.host, args.remote_root)
        stage_launcher(root, work, args.host, args.remote_root)
        worker(args.host, args.remote_root, args.phase)
        if args.phase == 'launcher-lock':
            launcher = load_builder().sibling_repo(root, 'game-launcher')
            run('scp', f'{args.host}:{args.remote_root}/source/game-launcher/Cargo.lock',
                str(launcher / 'Cargo.lock'))
        elif args.phase == 'launcher':
            artifacts = work / 'artifacts'
            artifacts.mkdir(exist_ok=True)
            run('scp', f'{args.host}:{args.remote_root}/target-launcher/x86_64-pc-windows-msvc/release/game-launcher.exe',
                str(artifacts / 'game-launcher-windows-x86_64.exe'))
    if args.phase in ('build', 'all'):
        bootstrap(args.host, args.remote_root)
        stage_source(root, work, args.host, args.remote_root)
        worker(args.host, args.remote_root, 'extension')
        worker(args.host, args.remote_root, 'runtime')
        fetch_artifacts(work, args.host, args.remote_root)
    if args.phase == 'artifacts':
        fetch_artifacts(work, args.host, args.remote_root)
    if args.phase in ('assemble', 'all'):
        assemble(root, work, args.host, args.remote_root)


if __name__ == '__main__':
    main()
