"""Private70391 CSV adapter for existing strict NPC importer; no canonical writes."""
from pathlib import Path
import argparse
import hashlib
import importlib.util
import json
import sqlite3

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--root', type=Path, required=True)
parser.add_argument('--source-world', type=Path, required=True)
parser.add_argument('--output', type=Path, required=True)
args = parser.parse_args()
ROOT = args.root.resolve()
SOURCE = args.source_world.resolve()
OUTPUT = args.output.resolve()
CSV = ROOT / 'data/db2/active'
spec = importlib.util.spec_from_file_location('npc_import', ROOT / 'scripts/import_npc_appearance.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
with sqlite3.connect(SOURCE.resolve().as_uri() + '?mode=ro', uri=True) as conn:
    selected = {row[0] for row in conn.execute('SELECT CreatureDisplayID FROM content_creature_template_model WHERE CreatureID IN (4484,3663,3879,3893,3898,3899,3900)')}
displays = {int(row['ID']): int(row['ExtendedDisplayInfoID']) for row in module.read_csv(CSV / 'CreatureDisplayInfo.csv') if int(row['ID']) in selected}
assert selected == displays.keys(), ('source-absent displays', selected - displays.keys())
extras = {int(row['ID']): (tuple(int(row[k]) for k in ['ID','DisplayRaceID','DisplaySexID','DisplayClassID','Flags','BakeMaterialResourcesID','HDBakeMaterialResourcesID']), None) for row in module.read_csv(CSV / 'CreatureDisplayInfoExtra.csv')}
options = {int(row['ID']): ((int(row['ChrCustomizationOptionID']), int(row['ChrCustomizationChoiceID'])), int(row['CreatureDisplayInfoExtraID'])) for row in module.read_csv(CSV / 'CreatureDisplayInfoOption.csv')}
geosets = {int(row['ID']): ((int(row['GeosetIndex']), int(row['GeosetValue'])), int(row['CreatureDisplayInfoID'])) for row in module.read_csv(CSV / 'CreatureDisplayInfoGeosetData.csv')}
models = module.load_model_paths(ROOT / 'data/cache/creature_display.sqlite', ROOT / 'data/community-listfile.csv', displays)
materials = {display: module.select_material(extras[extra][0], models[display]) for display, extra in displays.items() if extra}
textures = module.load_textures(CSV / 'TextureFileData.csv', None, set(materials.values()) - {0})
rows = module.join_appearances(displays, extras, options, geosets, models, textures)
merged = module.merge_existing_rows(ROOT / 'data/cache/npc_appearance.sqlite', rows)
module.write_database(OUTPUT, merged)
manifest = {
    'source': str(SOURCE), 'csv_root': str(CSV.resolve()),
    'selected_displays': sorted(selected),
    'counts': dict(zip(['appearances','choices','geosets','coverage'], map(len, rows))),
    'sources_sha256': {name: hashlib.sha256((CSV/name).read_bytes()).hexdigest() for name in ['CreatureDisplayInfo.csv','CreatureDisplayInfoExtra.csv','CreatureDisplayInfoOption.csv','CreatureDisplayInfoGeosetData.csv','TextureFileData.csv']},
    'output_sha256': hashlib.sha256(OUTPUT.read_bytes()).hexdigest(),
}
OUTPUT.with_suffix('.json').write_text(json.dumps(manifest, indent=2))
print(json.dumps(manifest, indent=2))
