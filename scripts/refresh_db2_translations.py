"""Regenerate legacy CSV Translation artifacts through bounded native imports.

One CSV per Godot process prevents concurrent large-table imports from exhausting
host memory. Uncompressed resources retain native lookup semantics. These are
legacy table artifacts, not the game's localized UI catalog.
"""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess

PROJECT = 'config_version=5\n[application]\nconfig/name="Private DB2 translations"\n[rendering]\nrenderer/rendering_method="gl_compatibility"\n'


def import_table(source, project, godot):
    if project.exists():
        shutil.rmtree(project)
    project.mkdir(parents=True)
    (project / 'project.godot').write_text(PROJECT)
    (project / source.name).symlink_to(source)
    (project / (source.name + '.import')).write_text(
        '[remap]\nimporter="csv_translation"\ntype="Translation"\n'
        f'[deps]\nsource_file="res://{source.name}"\n'
        '[params]\ncompress=0\ndelimiter=0\nunescape_keys=false\nunescape_translations=true\n')
    with (project / 'import.log').open('w') as log:
        result = subprocess.run([str(godot), '--headless', '--path', str(project), '--editor', '--import', '--quit'],
                                stdout=log, stderr=subprocess.STDOUT)
    if result.returncode:
        raise RuntimeError(f'{source.name}: native import exited {result.returncode}; inspect {project / "import.log"}')
    return sorted(project.glob('*.translation'))


def regenerate_translations(sources, output, godot):
    output = Path(output).resolve()
    sources = [Path(source).resolve() for source in sources]
    if any(source.parent == output for source in sources):
        raise ValueError('translation project must not overwrite its CSV input directory')
    output.mkdir(parents=True, exist_ok=True)
    (output / 'project.godot').write_text(PROJECT)
    for previous in output.glob('*.translation'):
        previous.unlink()
    projects = output / 'projects'
    projects.mkdir(exist_ok=True)
    (projects / '.gdignore').touch()
    artifacts = {}
    for source in sources:
        for path in import_table(source, projects / source.stem, godot):
            if path.name in artifacts:
                raise ValueError(f'duplicate translation artifact {path.name}')
            destination = output / path.name
            shutil.copy2(path, destination)
            artifacts[path.name] = hashlib.sha256(destination.read_bytes()).hexdigest()
        print(f'TRANSLATION {source.name} {len(artifacts)} resources', flush=True)
    receipt = {'sources': {source.name: hashlib.sha256(source.read_bytes()).hexdigest() for source in sources},
               'artifacts': artifacts, 'compression': False}
    (output / 'receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')
    return receipt
