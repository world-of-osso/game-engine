#!/usr/bin/env python3
"""Frozen Retail asset closure for Adventure Guide boss previews into a private package.

Seeds every positive JournalEncounterCreature display from the client's Journal cache.
Bytes come only from authenticated local CASC (`casc-local`); no canonical mutation.
Existing receipts in canonical data and in `--existing-package` directories are reused.
"""
import argparse
from pathlib import Path
import sqlite3

from closure_seeds import seed_displays
from stage_mount_model_assets import manifest, stage_closure, transitive_assets
from stage_trading_post_model_assets import casc_runner

FRAGMENT = 'model-asset-index-adventure-guide-boss-assets-20261013.json'


def seeds(journal):
    """(encounter, displayed preview, every positive creature display) in Guide order."""
    rows = {}
    query = 'SELECT JournalEncounterID, CreatureDisplayInfoID FROM JournalEncounterCreature ORDER BY OrderIndex, ID'
    for encounter, display in journal.execute(query):
        shown, displays = rows.setdefault(encounter, (display, []))
        if display > 0 and display not in displays:
            displays.append(display)
    return [(encounter, shown, sorted(displays)) for encounter, (shown, displays) in sorted(rows.items())]


def display_report(display, assets, receipts, failures):
    prefix = f'display {display} '
    roots = {key for key, asset in assets.items() if any(reason.startswith(prefix) for _, reason in asset['edges'])}
    missing = sorted(key for key in transitive_assets(roots, assets) if key not in receipts)
    reasons = [f'{fdid}.{kind}: {failures.get((fdid, kind), "no verified asset receipt")}' for fdid, kind in missing]
    if not roots:
        reasons.append('no sourced model or texture root')
    return reasons


def summarize(rows, assets, receipts, failures):
    displays = sorted({display for _, _, ds in rows for display in ds})
    verdicts = {display: display_report(display, assets, receipts, failures) for display in displays}
    encounters = []
    for encounter, shown, _ in rows:
        reasons = verdicts.get(shown, ['no sourced preview display']) if shown > 0 else ['no sourced preview display']
        encounters.append({'encounter': encounter, 'preview_display': shown,
                           'verified_preview_chain': not reasons, 'reasons': reasons})
    return {'displays': len(displays), 'verified_displays': sum(not reasons for reasons in verdicts.values()),
            'encounters_total': len(encounters), 'verified_encounters': sum(row['verified_preview_chain'] for row in encounters),
            'encounters': encounters,
            'display_reasons': {str(display): reasons for display, reasons in verdicts.items() if reasons}}


def seed_journal(rows):
    def seed(graph, catalogs):
        seed_displays(graph, catalogs, [display for _, _, ds in rows for display in ds])
        return lambda assets, receipts, failures: summarize(rows, assets, receipts, failures)
    return seed


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--data', type=Path, required=True, help='canonical data dir (read only)')
    parser.add_argument('--journal', type=Path, required=True, help='client cache/adventure_guide.sqlite')
    parser.add_argument('--install', type=Path, required=True)
    parser.add_argument('--casc-local', type=Path, required=True)
    parser.add_argument('--package', type=Path, required=True, help='private output package')
    parser.add_argument('--existing-package', type=Path, action='append', default=[])
    args = parser.parse_args()
    with sqlite3.connect(args.journal.resolve().as_uri() + '?mode=ro', uri=True) as journal:
        rows = seeds(journal)
    stage_closure(args.data, args.package, args.install, casc_runner(args.casc_local),
                  seed_journal(rows), 'every positive JournalEncounterCreature display; pinned Retail 69933',
                  FRAGMENT, args.existing_package)
    manifest(args.data, args.package)


if __name__ == '__main__':
    main()
