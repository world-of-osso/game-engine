"""Real Windows process-boundary proof for the extensionless launcher contract."""

import shutil
import subprocess
from pathlib import Path


def compile_fixture(source, destination):
    subprocess.run(['rustc', '--edition', '2024', '--target', 'x86_64-pc-windows-gnu',
                    '-', '-o', str(destination)], input=source, text=True, check=True)


def prove_entry(root, engine):
    fixture = root / 'entry proof with spaces'
    (fixture / 'runtime').mkdir(parents=True, exist_ok=True)
    subprocess.run(['rustc', '--edition', '2024', '--target', 'x86_64-pc-windows-gnu',
                    str(engine / 'scripts/windows-entry.rs'), '-o', str(fixture / 'entry.exe')], check=True)
    shutil.copy2(fixture / 'entry.exe', fixture / 'game-engine-windows-x86_64')
    compile_fixture('''fn main() {
        let arguments = std::env::args().skip(1).collect::<Vec<_>>().join("\\n");
        let runtime = std::env::current_exe().unwrap();
        let root = runtime.parent().unwrap().parent().unwrap();
        std::fs::write(root.join("arguments.txt"), arguments).unwrap();
        std::fs::write(root.join("asset-mode.txt"), std::env::var("GAME_ENGINE_ASSET_MODE").unwrap()).unwrap();
        let code = std::env::var("ENTRY_TEST_EXIT").unwrap_or_else(|_| "7".into());
        std::process::exit(code.parse().unwrap());
    }''', fixture / 'runtime/godot.exe')
    compile_fixture('''fn main() {
        let game_dir = std::path::PathBuf::from(std::env::args_os().nth(1).unwrap());
        let working_dir = game_dir.ancestors().last().unwrap();
        let mut child = std::process::Command::new(game_dir.join("game-engine-windows-x86_64"))
            .current_dir(working_dir).spawn().unwrap();
        assert_eq!(child.wait().unwrap().code(), Some(7));
    }''', fixture / 'launcher-proof.exe')
    subprocess.run([str(fixture / 'launcher-proof.exe'), str(fixture)], check=True)
    expected = ['--path', str(fixture / 'godot'), '--']
    assert (fixture / 'arguments.txt').read_text(encoding='utf-8').splitlines() == expected
    result = subprocess.run([str(fixture / 'game-engine-windows-x86_64'), '--char', 'Windows Space Æ'])
    assert result.returncode == 7
    assert (fixture / 'arguments.txt').read_text(encoding='utf-8').splitlines() == expected + ['--char', 'Windows Space Æ']
    assert (fixture / 'asset-mode.txt').read_text() == 'extracted-only'
    import os
    result = subprocess.run([str(fixture / 'game-engine-windows-x86_64')],
                            env={**os.environ, 'ENTRY_TEST_EXIT': '256'})
    assert result.returncode == 256, f'Windows exit status truncated to {result.returncode}'
    print('PASS: rooted launcher CWD, extensionless PE, spaces/Unicode arguments, relative runtime, child status, extracted-only')
