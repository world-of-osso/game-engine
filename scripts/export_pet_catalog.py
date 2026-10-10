#!/usr/bin/env python3
"""Join exported LOCAL CASC BattlePetSpecies/Creature tables for runtime consumers.

Usage: export_pet_catalog.py <db2 CSV directory> <output.json>
All readable species are retained. Missing creature names/displays stay absent
(empty/zero); runtime summoning must reject an unresolved display, not invent one.
"""
import csv
import json
from pathlib import Path
import sys


def read_rows(path):
    with path.open(newline="") as handle:
        return list(csv.DictReader(handle))


def join_catalog(species, creatures):
    by_id = {int(row["ID"]): row for row in creatures}
    catalog = []
    for row in species:
        creature = by_id.get(int(row["CreatureID"]))
        display = 0
        name = ""
        if creature is not None:
            name = creature["Name_lang"]
            display = next((int(creature[f"DisplayID_{i}"]) for i in range(4)
                            if int(creature[f"DisplayID_{i}"]) > 0), 0)
        catalog.append({
            "species_id": int(row["ID"]), "creature_id": int(row["CreatureID"]),
            "name": name, "display_id": display,
            "summon_spell_id": int(row["SummonSpellID"]),
            "icon_file_id": int(row["IconFileDataID"]),
            "family": int(row["PetTypeEnum"]), "flags": int(row["Flags"]),
            "description": row["Description_lang"], "source": row["SourceText_lang"],
        })
    return catalog


def main():
    source, output = map(Path, sys.argv[1:3])
    catalog = join_catalog(read_rows(source / "BattlePetSpecies.csv"),
                           read_rows(source / "Creature.csv"))
    output.write_text(json.dumps(catalog, ensure_ascii=False, indent=2) + "\n")
    unresolved = sum(not pet["display_id"] or not pet["name"] for pet in catalog)
    print(f"{len(catalog)} species, {unresolved} unresolved creature name/display")


if __name__ == "__main__":
    main()
