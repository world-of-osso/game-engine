#!/usr/bin/env python3
"""Frozen Retail asset closure for Trading Post previews into a private package.

Seeds every sourced PerksVendorItem preview the server serves: mount/pet creature
displays and transmog/set item displays. Bytes come only from authenticated local
CASC (`casc-local`); no canonical mutation. Existing receipts in canonical data and in
`--existing-package` directories are reused, never duplicated.
"""
import argparse
import json
import os
from pathlib import Path
import sqlite3
import subprocess

from closure_seeds import seed_displays, seed_item_displays
from stage_mount_model_assets import manifest, stage_closure, transitive_assets

LEARN_TRANSMOG_SET = 255
FRAGMENT = 'model-asset-index-trading-post-model-assets-20261012.json'


def vendor_rows(world):
    return [json.loads(payload) for (payload,) in world.execute('SELECT payload FROM perks_vendor_item ORDER BY ID')]


def creature_display(world, row):
    """Server catalog rule: authored override, else the granted mount/pet display."""
    if row['CreatureDisplayInfoID']:
        return row['CreatureDisplayInfoID']
    if row['PerksVendorCategoryID'] == 2:
        found = world.execute('SELECT COALESCE((SELECT d.CreatureDisplayInfoID FROM mount_x_display d WHERE d.MountID=m.ID AND d.PlayerConditionID=0 ORDER BY d.ID LIMIT 1),0) FROM item_x_item_effect x JOIN item_effect e ON e.ID=x.ItemEffectID JOIN mount m ON m.SourceSpellID=e.SpellID WHERE x.ItemID=? AND e.TriggerType=6 ORDER BY e.ID LIMIT 1', (row['ItemID'],)).fetchone()
    else:
        found = world.execute('SELECT s.display_id FROM item_x_item_effect x JOIN item_effect e ON e.ID=x.ItemEffectID JOIN battle_pet_species s ON s.summon_spell_id=e.SpellID WHERE x.ItemID=? AND e.TriggerType=6 ORDER BY e.ID LIMIT 1', (row['ItemID'],)).fetchone()
    return found[0] if found else 0


def preview_items(world, sets, row):
    if row['PerksVendorCategoryID'] == 1:
        return [row['ItemID']]
    found = world.execute('SELECT s.EffectMiscValue_0 FROM item_x_item_effect x JOIN item_effect e ON e.ID=x.ItemEffectID JOIN spell_effect s ON s.SpellID=e.SpellID WHERE x.ItemID=? AND s.DifficultyID=0 AND s.Effect=? ORDER BY e.ID LIMIT 1', (row['ItemID'], LEARN_TRANSMOG_SET)).fetchone()
    members = sets.get(found[0], {}).get('source_ids', []) if found else []
    return [item for (item,) in (world.execute('SELECT ItemID FROM item_modified_appearance WHERE ID=?', (source,)).fetchone() or (None,) for source in members) if item]


def seeds(world, sets):
    """(vendor id, category, 'creature'|'item', display ids) for each previewable row."""
    out = []
    for row in vendor_rows(world):
        category = row['PerksVendorCategoryID']
        if category in (2, 3):
            display = creature_display(world, row)
            out.append((row['ID'], category, 'creature', [display] if display else []))
        elif category in (1, 8):
            displays = []
            for item in preview_items(world, sets, row):
                found = world.execute('SELECT a.ItemDisplayInfoID FROM item_modified_appearance m JOIN item_appearance a ON a.ID=m.ItemAppearanceID WHERE m.ItemID=? AND m.ItemAppearanceModifierID=0 ORDER BY m.OrderIndex,m.ID LIMIT 1', (item,)).fetchone()
                if found and found[0]:
                    displays.append(found[0])
            out.append((row['ID'], category, 'item', displays))
    return out


def summarize(rows, assets, receipts, failures):
    def roots(kind, display):
        prefix = f'{"item display" if kind == "item" else "display"} {display} '
        return {key for key, asset in assets.items() if any(reason.startswith(prefix) for _, reason in asset['edges'])}

    report = []
    for vendor, category, kind, displays in rows:
        required = transitive_assets(set().union(*[roots(kind, display) for display in displays]), assets) if displays else set()
        missing = sorted(key for key in required if key not in receipts)
        reasons = [f'{fdid}.{k}: {failures.get((fdid, k), "no verified asset receipt")}' for fdid, k in missing]
        if not displays:
            reasons.append('no sourced preview display')
        report.append({'vendor_item': vendor, 'category': category, 'kind': kind, 'displays': displays,
                       'required_assets': len(required), 'verified_asset_chain': not reasons, 'reasons': reasons})
    by_category = {}
    for row in report:
        total, ok = by_category.get(row['category'], (0, 0))
        by_category[row['category']] = (total + 1, ok + row['verified_asset_chain'])
    return {'rows': len(report), 'verified_rows': sum(row['verified_asset_chain'] for row in report),
            'by_category': {str(key): {'rows': total, 'verified': ok} for key, (total, ok) in sorted(by_category.items())},
            'display_links': report}


def seed_trading_post(rows):
    def seed(graph, catalogs):
        seed_displays(graph, catalogs, [d for _, _, kind, ds in rows if kind == 'creature' for d in ds])
        seed_item_displays(graph, catalogs, {d for _, _, kind, ds in rows if kind == 'item' for d in ds})
        return lambda assets, receipts, failures: summarize(rows, assets, receipts, failures)
    return seed


def casc_runner(casc_local):
    # Resolver caches/extraction roots come from the caller's private ASSET_RESOLVER_* env.
    def extract(fdids, output):
        return subprocess.run([str(casc_local), *map(str, fdids), '-o', str(output)],
                              capture_output=True, text=True, check=False, env=dict(os.environ, WOW_PRODUCT='wow'))
    return extract


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--data', type=Path, required=True, help='canonical data dir (read only)')
    parser.add_argument('--world', type=Path, required=True, help='world.db holding perks_vendor_* tables')
    parser.add_argument('--transmog', type=Path, required=True, help='game-server scripts/reference/retail-12.1.0.69933/transmog.json')
    parser.add_argument('--install', type=Path, required=True)
    parser.add_argument('--casc-local', type=Path, required=True)
    parser.add_argument('--package', type=Path, required=True, help='private output package')
    parser.add_argument('--existing-package', type=Path, action='append', default=[])
    args = parser.parse_args()
    sets = {row['set_id']: row for row in json.loads(args.transmog.read_text())['sets']}
    with sqlite3.connect(args.world.resolve().as_uri() + '?mode=ro', uri=True) as world:
        rows = seeds(world, sets)
    stage_closure(args.data, args.package, args.install, casc_runner(args.casc_local),
                  seed_trading_post(rows), 'every sourced PerksVendorItem mount/pet/transmog/set preview; pinned Retail 69933',
                  FRAGMENT, args.existing_package)
    manifest(args.data, args.package)


if __name__ == '__main__':
    main()
