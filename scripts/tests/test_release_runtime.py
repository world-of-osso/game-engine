import hashlib
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


class ReleaseRuntimeTest(unittest.TestCase):
    def test_dry_run_imports_with_editor_and_packages_template(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            scripts = root / 'scripts'
            (scripts / 'godot').mkdir(parents=True)
            source = Path(__file__).parents[1]
            for name in ('player_bundle.py', 'import_project.py'):
                shutil.copy2(source / name, scripts / name)
            for name in ('pinned.sh', 'verify-runtime.py'):
                shutil.copy2(source / 'godot' / name, scripts / 'godot' / name)
            shutil.copy2(source.parent / 'deploy.sh', root / 'deploy.sh')
            (root / 'data').mkdir()
            (root / 'godot').mkdir()
            (root / 'godot/project.godot').write_text('project')
            subprocess.run(['git', 'init', '-q', str(root)], check=True)
            subprocess.run(['git', '-C', str(root), 'add', 'godot'], check=True)
            (scripts / 'depot-build.py').write_text('''from pathlib import Path
p = Path(__file__).resolve().parents[1] / 'target/release/libgame_engine_godot.so'
p.parent.mkdir(parents=True, exist_ok=True)
p.write_bytes(b'extension')
''')
            (scripts / 'windows-release.py').write_text('pass\n')
            (scripts / 'mapped_deploy.py').write_text('''from pathlib import Path
import sys
assert '--destination' not in sys.argv
generated = Path(sys.argv[sys.argv.index('--generated') + 1])
assert (generated / 'runtime/godot').read_bytes() == b'template'
assert (generated / 'godot/editor-ran').read_text() == 'yes'
assert not (generated / 'runtime/editor').exists()
''')
            tools = root / 'tools'
            tools.mkdir()
            cargo = tools / 'cargo'
            cargo.write_text('#!/usr/bin/python3\n')
            cargo.chmod(0o755)
            version = '4.7.2-pr123946-pr123546'
            cache = root / 'cache/game-engine/godot' / version
            cache.mkdir(parents=True)
            editor = cache / f'godot-{version}'
            editor.write_text('''#!/usr/bin/python3
from pathlib import Path
import sys
Path(sys.argv[sys.argv.index('--path') + 1], 'editor-ran').write_text('yes')
''')
            editor.chmod(0o755)
            runtime = cache / f'godot-template-release-{version}'
            runtime.write_bytes(b'template')
            runtime.chmod(0o755)
            (scripts / 'godot' / f'godot-{version}.sha512').write_text(hashlib.sha512(editor.read_bytes()).hexdigest() + '\n')
            (scripts / 'godot' / f'godot-{version}-template-release.sha512').write_text(hashlib.sha512(b'template').hexdigest() + '\n')
            env = dict(os.environ, XDG_CACHE_HOME=str(root / 'cache'),
                       PATH=str(tools) + os.pathsep + os.environ['PATH'])
            result = subprocess.run([str(root / 'deploy.sh'), '--dry-run'], env=env, text=True, capture_output=True)
            self.assertEqual(result.returncode, 0, result.stderr)

    def test_builder_installs_checked_release_template(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            scripts = root / 'scripts/godot'
            scripts.mkdir(parents=True)
            original = Path(__file__).parents[1] / 'godot'
            for name in ('build-patched-godot.sh', 'pinned.sh'):
                shutil.copy2(original / name, scripts / name)
            version = '4.7.2-pr123946-pr123546'
            (scripts / f'godot-{version}-template-release.sha512').write_text(hashlib.sha512(b'template').hexdigest() + '\n')
            work = root / 'work'
            sdk = work / 'x86_64-godot-linux-gnu_sdk-buildroot'
            (sdk / 'bin').mkdir(parents=True)
            (sdk / '.relocated').touch()
            (work / 'accesskit/accesskit-c').mkdir(parents=True)
            tools = root / 'tools'
            tools.mkdir()
            git = tools / 'git'
            git.write_text('''#!/usr/bin/python3
import pathlib, sys
if 'clone' in sys.argv:
    pathlib.Path(sys.argv[-1]).mkdir(parents=True)
elif 'rev-parse' in sys.argv:
    print('ed1daf0bf001b61586d9930840f2f1394092c079')
''')
            git.chmod(0o755)
            uvx = tools / 'uvx'
            uvx.write_text('''#!/usr/bin/python3
import pathlib, sys
if 'target=template_release' not in sys.argv:
    raise SystemExit('wrong Godot target')
output = pathlib.Path.cwd() / 'bin/godot.linuxbsd.template_release.x86_64'
output.parent.mkdir(parents=True)
output.write_bytes(b'template')
''')
            uvx.chmod(0o755)
            env = dict(os.environ, XDG_CACHE_HOME=str(root / 'cache'), GODOT_BUILD_DIR=str(work),
                       PATH=str(tools) + os.pathsep + os.environ['PATH'])
            result = subprocess.run([str(scripts / 'build-patched-godot.sh'), '--template-release', '--jobs', '1'],
                                    env=env, text=True, capture_output=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            runtime = root / 'cache/game-engine/godot' / version / f'godot-template-release-{version}'
            self.assertEqual(runtime.read_bytes(), b'template')

    def test_runtime_checksum_accepts_exact_binary_and_rejects_mutation(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            runtime = root / 'godot-template'
            checksum = root / 'runtime.sha512'
            runtime.write_bytes(b'template')
            runtime.chmod(0o755)
            checksum.write_text(hashlib.sha512(b'template').hexdigest() + '\n')
            command = [sys.executable, str(Path(__file__).parents[1] / 'godot/verify-runtime.py'),
                       str(runtime), str(checksum)]
            good = subprocess.run(command, text=True, capture_output=True)
            self.assertEqual(good.returncode, 0, good.stderr)
            self.assertEqual(good.stdout.strip(), str(runtime))
            runtime.write_bytes(b'mutated!')
            bad = subprocess.run(command, text=True, capture_output=True)
            self.assertNotEqual(bad.returncode, 0)
            self.assertIn('godot-template', bad.stderr)


if __name__ == '__main__':
    unittest.main()
