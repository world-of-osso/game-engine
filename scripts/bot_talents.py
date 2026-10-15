"""Read sourced TraitTreeLoadout order without inventing talent choices."""
import argparse
import json
from pathlib import Path

from bot_rotation import read_rows


def select_loadout(directory, spec, loadout):
    choices = [row for row in read_rows(directory, "TraitTreeLoadout")
               if int(row["ChrSpecializationID"]) == spec]
    if loadout is None:
        if len(choices) != 1:
            raise ValueError(f"Spec {spec} needs an explicit loadout from {[row['ID'] for row in choices]}")
        return choices[0]
    selected = next((row for row in choices if int(row["ID"]) == loadout), None)
    if selected is None:
        raise ValueError(f"Loadout {loadout} does not belong to specialization {spec}")
    return selected


def resolve_entry(row, links, entries):
    node = int(row["SelectedTraitNodeID"])
    choices = {int(link["TraitNodeEntryID"]) for link in links if int(link["TraitNodeID"]) == node}
    selected = int(row["SelectedTraitNodeEntryID"])
    if selected == 0:
        if len(choices) != 1:
            raise ValueError(f"Loadout row {row['ID']} does not source a choice for node {node}")
        selected = next(iter(choices))
    if selected not in choices:
        raise ValueError(f"Loadout row {row['ID']} selects entry {selected} outside node {node}")
    if not 0 < int(row["NumPoints"]) <= int(entries[selected]["MaxRanks"]):
        raise ValueError(f"Loadout row {row['ID']} has invalid target rank")
    return selected


def load_plan(directory, spec, loadout=None):
    selected = select_loadout(directory, spec, loadout)
    rows = [row for row in read_rows(directory, "TraitTreeLoadoutEntry")
            if row["TraitTreeLoadoutID"] == selected["ID"]]
    if not rows:
        raise ValueError(f"Loadout {selected['ID']} has no sourced entries")
    links = list(read_rows(directory, "TraitNodeXTraitNodeEntry"))
    entries = {int(row["ID"]): row for row in read_rows(directory, "TraitNodeEntry")}
    definitions = {int(row["ID"]): row for row in read_rows(directory, "TraitDefinition")}
    steps = []
    for row in sorted(rows, key=lambda row: int(row["OrderIndex"])):
        entry = resolve_entry(row, links, entries)
        node = int(row["SelectedTraitNodeID"])
        spell = int(definitions[int(entries[entry]["TraitDefinitionID"])]["SpellID"])
        steps.append({"node": node, "entry": entry, "spell": spell,
                      "rank": int(row["NumPoints"]), "order": int(row["OrderIndex"]),
                      "frame": f"TalentNode{node}Entry{entry}Spell{spell}Button", "source": row})
    return {"spec": spec, "loadout": int(selected["ID"]), "tree": int(selected["TraitTreeID"]),
            "steps": steps, "source": "installed TraitTreeLoadout/Entry order; NumPoints is cumulative target rank"}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("db2", type=Path)
    parser.add_argument("spec", type=int)
    parser.add_argument("output", type=Path)
    parser.add_argument("--loadout", type=int)
    args = parser.parse_args()
    args.output.write_text(json.dumps(load_plan(args.db2, args.spec, args.loadout), indent=2) + "\n")


if __name__ == "__main__":
    main()
