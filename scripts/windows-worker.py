#!/usr/bin/env python3
"""Native Windows GNU release worker, uploaded by windows-release.py."""

import argparse
import hashlib
import os
import runpy
import shutil
import subprocess
from pathlib import Path

GODOT_COMMIT = 'ed1daf0bf001b61586d9930840f2f1394092c079'


def run(*arguments, cwd=None):
    subprocess.run(arguments, cwd=cwd, check=True)


def configure(root):
    mingw = Path(shutil.which('gcc')).parent
    os.environ.update(
        HOME=str(Path.home()), LIBCLANG_PATH=r'C:\Program Files\LLVM\bin',
        MINGW_PREFIX=str(mingw.parent), KTX_CACHE_DIR=str(Path.home() / '.woo-ktx'),
        CARGO_BUILD_JOBS='4', PYTHONHASHSEED='0', BUILD_NAME='pr123546-pr123391-font-atlas',
        RUSTUP_TOOLCHAIN='1.98.1',
    )
    clang = Path(os.environ['LIBCLANG_PATH']) / 'clang.exe'
    resource = subprocess.check_output([str(clang), '-print-resource-dir'], text=True).strip()
    headers = mingw.parent / 'x86_64-w64-mingw32/include'
    # libclang cannot discover MinGW's user-installed headers from an MSVC-host Python.
    os.environ['BINDGEN_EXTRA_CLANG_ARGS_x86_64_pc_windows_gnu'] = (
        f'-resource-dir "{Path(resource).as_posix()}" -isystem "{headers.as_posix()}"'
    )
    os.environ['PATH'] = str(mingw) + os.pathsep + str(clang.parent) + os.pathsep + os.environ['PATH']
    return mingw


def build_runtime(root, uv, engine):
    source = root / 'godot-source'
    if not source.exists():
        run('git', 'clone', '--depth', '1', '--branch', '4.7.2-stable',
            'https://github.com/godotengine/godot.git', str(source))
    revision = subprocess.check_output(['git', '-C', str(source), 'rev-parse', 'HEAD'], text=True).strip()
    if revision != GODOT_COMMIT:
        raise RuntimeError(f'Godot source revision {revision}, expected {GODOT_COMMIT}')
    for patch_name in ('pr123546-classdb-default-values-race.patch', 'pr123391-render-device-thread-ownership.patch',
                       'font-atlas-snapshot.patch'):
        patch = engine / 'scripts/godot' / patch_name
        # A repeat build must prove the exact patch already applied, not silently skip errors.
        applied = subprocess.run(['git', '-C', str(source), 'apply', '--reverse', '--check', str(patch)], capture_output=True)
        if applied.returncode:
            run('git', '-C', str(source), 'apply', '--check', str(patch))
            run('git', '-C', str(source), 'apply', str(patch))
    for target in ('editor', 'template_release'):
        options = ['disable_path_overrides=no'] if target == 'template_release' else []
        run(str(uv), 'run', '--python', '3.12.12', '--with', 'scons==4.10.1', 'scons',
            '-j4', 'platform=windows', 'arch=x86_64', f'target={target}', *options,
            'use_mingw=yes', 'production=yes', 'accesskit=no', 'd3d12=no', 'winrt=no', cwd=source)
    runtime = root / 'artifacts/godot.exe'
    runtime.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source / 'bin/godot.windows.template_release.x86_64.exe', runtime)
    digest = hashlib.sha512(runtime.read_bytes()).hexdigest()
    expected = (engine / 'scripts/godot/godot-4.7.2-pr123546-pr123391-windows-template.sha512').read_text().strip()
    if digest != expected:
        raise RuntimeError(f'Windows Godot runtime checksum {digest}, expected {expected}')
    (runtime.parent / 'godot.sha512').write_text(digest + '\n')


def build_extension(root, engine):
    run('cargo', 'build', '--locked', '--release', '--target', 'x86_64-pc-windows-gnu',
        '--manifest-path', str(engine / 'godot/Cargo.toml'), '--target-dir', str(root / 'target'),
        '-p', 'game-engine-godot', '--lib', cwd=engine)
    artifacts = root / 'artifacts'
    artifacts.mkdir(exist_ok=True)
    shutil.copy2(root / 'target/x86_64-pc-windows-gnu/release/game_engine_godot.dll', artifacts)
    run('rustc', '--edition', '2024', '--target', 'x86_64-pc-windows-gnu', '-O',
        str(engine / 'scripts/windows-entry.rs'), '-o', str(artifacts / 'game-engine-windows-x86_64.exe'))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, required=True)
    parser.add_argument('--phase', choices=('extension', 'runtime', 'import', 'entry-test', 'ipc-test', 'launcher-test', 'launcher', 'launcher-lock', 'shutdown-test'), required=True)
    args = parser.parse_args()
    root = args.root.resolve()
    engine = root / 'source/game-engine'
    uv = root / 'tools/uv/uv.exe'
    configure(root)
    if args.phase == 'shutdown-test':
        # Core's other unit fixtures use Unix symlinks; this pure-std module can be
        # tested natively without compiling those unrelated platform fixtures.
        test = root / 'target/worker-shutdown-test.exe'
        run('rustc', '--test', '--edition', '2024', '--target', 'x86_64-pc-windows-gnu',
            str(engine / 'godot/core/src/worker_shutdown.rs'), '-o', str(test))
        run(str(test))
    elif args.phase == 'launcher-lock':
        launcher = root / 'source/game-launcher'
        run('cargo', 'update', '--package', 'game-launcher',
            '--manifest-path', str(launcher / 'Cargo.toml'), cwd=launcher)
    elif args.phase in ('launcher', 'launcher-test'):
        launcher = root / 'source/game-launcher'
        command = 'test' if args.phase == 'launcher-test' else 'build'
        arguments = ['--lib'] if command == 'test' else ['--features', 'desktop']
        run('cargo', command, '--locked', '--release', '--no-default-features',
            '--target', 'x86_64-pc-windows-msvc', '--manifest-path', str(launcher / 'Cargo.toml'),
            '--target-dir', str(root / 'target-launcher'), *arguments, cwd=launcher)
    elif args.phase == 'entry-test':
        runpy.run_path(str(engine / 'scripts/windows-entry-test.py'))['prove_entry'](root, engine)
    elif args.phase == 'ipc-test':
        run('cargo', 'test', '--release', '--target', 'x86_64-pc-windows-gnu',
            '--manifest-path', str(engine / 'scripts/windows-ipc-test/Cargo.toml'),
            '--target-dir', str(root / 'target'), cwd=engine)
    elif args.phase == 'extension':
        build_extension(root, engine)
    elif args.phase == 'runtime':
        build_runtime(root, uv, engine)
    else:
        bundle = root / 'player'
        os.environ['GAME_ENGINE_ASSET_MODE'] = 'extracted-only'
        editor = root / 'godot-source/bin/godot.windows.editor.x86_64.exe'
        run(str(editor), '--headless', '--path', str(bundle / 'godot'), '--import')
        shutil.rmtree(bundle / 'godot/.godot/editor', ignore_errors=True)


if __name__ == '__main__':
    main()
