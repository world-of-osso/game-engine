"""Regenerate legacy root CSV Translation artifacts in an isolated Godot project.

Uncompressed resources preserve native lookup semantics without the expensive
OptimizedTranslation compressor (large ItemSparse import exceeded five minutes).
These artifacts are not the game's localized UI catalog.
"""
import hashlib
import json
from pathlib import Path
import subprocess


def regenerate_translations(sources, output, godot):
    output = Path(output).resolve()
    sources = [Path(source).resolve() for source in sources]
    if any(source.parent == output for source in sources):
        raise ValueError('translation project must not overwrite its CSV input directory')
    output.mkdir(parents=True, exist_ok=True)
    (output / 'project.godot').write_text('config_version=5\n[application]\nconfig/name="Private DB2 translations"\n[rendering]\nrenderer/rendering_method="gl_compatibility"\n')
    for previous in output.glob('*.translation'):
        previous.unlink()
    for source in sources:
        link = output / source.name
        if link.is_symlink():
            link.unlink()
        elif link.exists():
            raise ValueError(f'refuse replacing translation input {link}')
        link.symlink_to(source)
        (output / (source.name + '.import')).write_text(
            '[remap]\nimporter="csv_translation"\ntype="Translation"\n'
            f'[deps]\nsource_file="res://{source.name}"\n'
            '[params]\ncompress=0\ndelimiter=0\nunescape_keys=false\nunescape_translations=true\n')
    with (output / 'import.log').open('w') as log:
        result = subprocess.run([str(godot), '--headless', '--path', str(output), '--editor', '--import', '--quit'],
                                stdout=log, stderr=subprocess.STDOUT)
    if result.returncode:
        raise RuntimeError(f'native translation import exited {result.returncode}; inspect {output / "import.log"}')
    receipt = {'sources': {source.name: hashlib.sha256(source.read_bytes()).hexdigest() for source in sources},
               'artifacts': {path.name: hashlib.sha256(path.read_bytes()).hexdigest() for path in sorted(output.glob('*.translation'))},
               'compression': False}
    (output / 'receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')
    return receipt
