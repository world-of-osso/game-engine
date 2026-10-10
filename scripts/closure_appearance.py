"""CreatureDisplayInfoExtra asset joins from local DB2s, not cache coverage guesses.

WoWDBDefs CreatureDisplayInfoExtra, CreatureDisplayInfoOption,
NPCModelItemSlotDisplayInfo and ChrCustomization*.dbd. Bake selection agrees
with scripts/import_npc_appearance.py::select_material: actual model path
ending _hd selects HD even when zero; never substitute the SD bake.
"""

from collections import defaultdict
from pathlib import PurePosixPath

from closure_seeds import number, seed_displays, seed_item_displays


class DisplayAppearanceReferences:
    def __init__(self, graph, catalogs):
        self.graph, self.catalogs = graph, catalogs
        self.indices, self.groups = {}, {}
        self.visited, self.item_displays, self.seeded_items = set(), set(), set()

    def index(self, table):
        if table not in self.indices:
            self.indices[table] = self.catalogs.index(table)
        return self.indices[table]

    def grouped(self, table, field):
        key = (table, field)
        if key not in self.groups:
            rows = defaultdict(list)
            for row in self.catalogs.rows(table):
                rows[number(row, field)].append(row)
            self.groups[key] = rows
        return self.groups[key]

    def required(self, table, identity):
        row = self.index(table).get(identity)
        if row is None:
            self.graph.issue("missing_metadata_row", f"{table} ID={identity}")
        return row

    def expand(self, display, extra_id, model_fdid):
        if display in self.visited:
            return
        self.visited.add(display)
        extra = self.required("CreatureDisplayInfoExtra", extra_id)
        if extra is None:
            return
        self.seed_bake(display, extra_id, extra, model_fdid)
        choices = {
            number(row, "ChrCustomizationChoiceID")
            for row in self.grouped(
                "CreatureDisplayInfoOption", "CreatureDisplayInfoExtraID"
            )[extra_id]
        } - {0}
        self.seed_choices(choices)
        self.item_displays.update(
            number(row, "ItemDisplayInfoID")
            for row in self.grouped("NPCModelItemSlotDisplayInfo", "NpcModelID")[
                extra_id
            ]
        )
        self.graph.resolve(
            "display_extended_appearance",
            None,
            "resolved",
            f"display {display} Extra {extra_id}: authored bake, {len(choices)} choices and NPC item-display resources; metadata hashing does not authenticate cache/runtime profile coverage",
        )

    def seed_bake(self, display, extra_id, extra, model_fdid):
        logical = self.graph.logical_path(model_fdid)
        if not logical:
            self.graph.issue(
                "appearance_model_path",
                f"display {display} model FDID {model_fdid}: HD/SD selector has no runtime logical path",
            )
            return
        hd = PurePosixPath(logical).stem.endswith("_hd")
        field = "HDBakeMaterialResourcesID" if hd else "BakeMaterialResourcesID"
        resource = number(extra, field)
        if resource:
            self.seed_material(resource, f"display {display} Extra {extra_id} {field}")
        else:
            self.graph.resolve(
                "npc_bake",
                model_fdid,
                "not_needed",
                f"display {display} {field}=0; select_material does not substitute other bake",
            )

    def seed_material(self, resource, reason):
        rows = self.grouped("TextureFileData", "MaterialResourcesID")[resource]
        if not rows:
            self.graph.issue(
                "missing_metadata_row", f"{reason} MaterialResourcesID={resource}"
            )
        for row in rows:
            self.graph.add(number(row, "FileDataID"), "blp", reason)

    def seed_choices(self, choices, mode="selected", require_rows=False):
        elements = self.grouped("ChrCustomizationElement", "ChrCustomizationChoiceID")
        for choice in sorted(choices):
            if require_rows and not elements[choice]:
                self.graph.issue(
                    "missing_metadata_row",
                    f"ChrCustomizationElement choice={choice}: legacy cache requires raw effect row",
                )
            for row in elements[choice]:
                related = number(row, "RelatedChrCustomizationChoiceID")
                if related and related not in choices and mode != "all":
                    continue
                reason = f"NPC customization choice {choice} element {row['ID']}"
                material_id = number(row, "ChrCustomizationMaterialID")
                if material_id:
                    material = self.required("ChrCustomizationMaterial", material_id)
                    if material:
                        self.seed_material(
                            number(material, "MaterialResourcesID"), reason
                        )
                self.seed_model(
                    row,
                    "ChrCustomizationSkinnedModelID",
                    "ChrCustomizationSkinnedModel",
                    "CollectionsFileDataID",
                    reason,
                )
                conditional_id = number(row, "ChrCustomizationCondModelID")
                if conditional_id:
                    conditional = self.required(
                        "ChrCustomizationCondModel", conditional_id
                    )
                    if conditional:
                        model_id = number(conditional, "CreatureModelDataID")
                        model = self.required("CreatureModelData", model_id)
                        if model:
                            self.graph.add(
                                number(model, "FileDataID"),
                                "m2",
                                f"{reason} conditional model {model_id}",
                            )
                display_id = number(row, "ChrCustomizationDisplayInfoID")
                if display_id:
                    other = self.required("ChrCustomizationDisplayInfo", display_id)
                    if other:
                        seed_displays(
                            self.graph,
                            self.catalogs,
                            [number(other, "CreatureDisplayInfoID")],
                        )
                voice = number(row, "ChrCustomizationVoiceID")
                if voice:
                    self.graph.issue(
                        "customization_voice_edges",
                        f"{reason} voice {voice}: unit voice metadata not joined",
                    )
                # Geoset/bone/AnimKit/ParticleColor/item-geo operations are numeric,
                # not filenames/FDIDs. Model bytes contain their external animations.

    def seed_model(self, row, field, table, fdid_field, reason):
        identity = number(row, field)
        if identity:
            model = self.required(table, identity)
            if model:
                self.graph.add(
                    number(model, fdid_field), "m2", f"{reason} {table} {identity}"
                )

    def flush_items(self):
        wanted = self.item_displays - self.seeded_items - {0}
        if wanted:
            self.seeded_items.update(wanted)
            seed_item_displays(self.graph, self.catalogs, wanted)
