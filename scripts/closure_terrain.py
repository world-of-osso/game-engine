"""ADT auxiliary identities from chunks and pinned local DB2 exports.

Layouts: core/asset/adt_format/{adt_tex_water,adt_tex,adt/parsing}.rs;
joins: core/{ground_detail,liquid_data}.rs. MTXF contains flags, NOT FDIDs.
Resolution evidence replaces blanket MCNK/MH2O warnings, not missing bytes.
"""

from collections import defaultdict
import struct

from asset_closure import chunks, integers, records, u32
from closure_seeds import Catalogs, number


class TerrainReferences:
    def __init__(self, graph):
        self.graph = graph
        self.catalogs = Catalogs(graph)
        self.indices = {}
        self.groups = {}

    def index(self, name):
        if name not in self.indices:
            self.indices[name] = self.catalogs.index(name)
        return self.indices[name]

    def grouped(self, name, key):
        if name not in self.groups:
            groups = defaultdict(list)
            for row in self.catalogs.rows(name):
                groups[number(row, key)].append(row)
            self.groups[name] = groups
        return self.groups[name]

    def require(self, name, key, parent, code="terrain_auxiliary_edges"):
        row = self.index(name).get(key)
        if row is None:
            self.graph.issue(
                code, f"{name} ID={key}: referenced metadata row absent", parent
            )
        return row

    def optional_ground_row(self, name, key, parent):
        row = self.index(name).get(key)
        source = self.graph.seeds.get("table_paths", {}).get(
            name, f"db2/{self.graph.metadata_build}/{name}.csv"
        )
        if row is None and source in self.graph.inputs:
            self.graph.resolve(
                "terrain_optional_effect",
                parent,
                "not_needed",
                f"{name} ID={key} absent from hashed pinned CSV; core ground_detail.rs GroundEffects::texture / native ground_detail.rs model_fdids skip absent optional rows before any model request; metadata completeness is not certified",
            )
        elif row is None:
            self.graph.issue(
                "terrain_auxiliary_edges",
                f"{name} ID={key}: metadata file absent",
                parent,
            )
        return row

    def ground_effect(self, effect, parent):
        if effect in {0, 0xFFFF, 0xFFFFFFFF}:
            return False
        row = self.optional_ground_row("GroundEffectTexture", effect, parent)
        if row is None:
            return False
        needed = False
        for column in range(4):
            doodad = number(row, f"DoodadID_{column}")
            if not doodad:
                continue
            target = self.optional_ground_row("GroundEffectDoodad", doodad, parent)
            if target is not None:
                model = number(target, "ModelFileID")
                self.graph.add(
                    model,
                    "m2",
                    f"MCLY effect {effect} -> GroundEffectDoodad {doodad}",
                    parent,
                )
                needed = needed or bool(model)
        return needed

    def liquid(
        self,
        liquid_type,
        liquid_object,
        parent,
        source="MH2O",
        issue_code="terrain_auxiliary_edges",
    ):
        if liquid_object >= 42:
            # Runtime deliberately uses the instance type for objects omitted by DB2
            # (including ocean object42); this is the existing liquid_data.rs rule.
            obj = self.index("LiquidObject").get(liquid_object)
            if obj is not None:
                liquid_type = number(obj, "LiquidTypeID")
        row = self.require("LiquidType", liquid_type, parent, issue_code)
        if row is None:
            return
        material = number(row, "MaterialID")
        self.require("LiquidMaterial", material, parent, issue_code)
        for texture in self.grouped("LiquidTypeXTexture", "LiquidTypeID").get(
            liquid_type, []
        ):
            self.graph.add(
                number(texture, "FileDataID"),
                "blp",
                f"{source} LiquidType {liquid_type} texture {texture['ID']}",
                parent,
            )
        if material in {2, 4}:
            self.graph.add(
                768431, "blob", f"LiquidMaterial {material} magma noise volume", parent
            )
        if material == 18:
            for fdid in [1797551, 1844666]:
                self.graph.add(
                    fdid, "blp", "LiquidMaterial18 environment/shore foam", parent
                )
        if material == 130:
            # The implemented renderer explicitly borrows legacy LiquidType5;
            # retain the authored PBR inputs AND those runtime dependencies.
            self.liquid(5, 0, parent, source, issue_code)

    def liquid_layers(self, payload):
        if len(payload) < 256 * 12:
            raise ValueError("MH2O header requires 256 12-byte chunk records")
        found = set()
        count = 0
        for offset in range(0, 256 * 12, 12):
            start, layers, _ = struct.unpack_from("<III", payload, offset)
            if layers and (start < 256 * 12 or start + layers * 24 > len(payload)):
                raise ValueError(
                    f"MH2O instances out of bounds at {start} count {layers}"
                )
            for index in range(layers):
                found.add(struct.unpack_from("<HH", payload, start + index * 24))
            count += layers
        return count, found

    def nested_references(self, payload, root):
        if root and payload:
            if len(payload) < 128:
                raise ValueError("root MCNK header requires 128 bytes")
            payload = payload[128:]
        effects, sounds, unknown, ignored = set(), set(), set(), set()
        geometry = {
            "MCVT",
            "MCNR",
            "MCCV",
            "MCLV",
            "MCSH",
            "MCAL",
            "MCRF",
            "MCRD",
            "MCRW",
            "MCBB",
            "MCDD",
            "MCMT",
            "MCQB",
        }
        for tag, block in chunks(payload, True):
            if tag == "MCLY":
                for row in records(block, 16):
                    effects.add(u32(row, 12))
            elif tag in {"MCSE", "ESCM"}:
                for row in records(block, 28):
                    sounds.add(u32(row))
            elif tag in {"MCLQ", "MPTX"}:
                ignored.add(tag)
            elif tag not in geometry:
                unknown.add(tag)
        return effects, sounds, unknown, ignored

    def sound(self, kit, parent):
        if not kit:
            return
        rows = self.grouped("SoundKitEntry", "SoundKitID").get(kit)
        if rows is None:
            self.graph.issue(
                "terrain_auxiliary_edges",
                f"MCSE SoundKit {kit}: entries absent",
                parent,
            )
            return
        for row in rows:
            fdid = number(row, "FileDataID")
            kind = self.graph.paths.get(fdid, "").rsplit(".", 1)[-1]
            if kind not in {"ogg", "mp3", "wav"}:
                kind = "audio"
            self.graph.add(fdid, kind, f"MCSE SoundKit {kit} entry {row['ID']}", parent)

    def expand(self, fdid, stream):
        table = dict(stream)
        effects, sounds, unknown, ignored = set(), set(), set(), set()
        root = "MHDR" in table
        mcnks = 0
        for tag, payload in stream:
            if tag == "MCNK":
                e, s, u, unused = self.nested_references(payload, root)
                effects.update(e)
                sounds.update(s)
                unknown.update(u)
                ignored.update(unused)
                mcnks += 1
        before_mcnk = len(self.graph.unresolved)
        needs_models = False
        for effect in sorted(effects):
            needs_models = self.ground_effect(effect, fdid) or needs_models
        for kit in sorted(sounds):
            self.sound(kit, fdid)
        for tag in sorted(unknown):
            self.graph.issue(
                "terrain_auxiliary_edges",
                f"MCNK {tag}: external reference semantics unproved",
                fdid,
            )
        for tag in sorted(ignored):
            self.graph.resolve(
                "terrain_unused_chunk",
                fdid,
                "not_needed",
                f"MCNK {tag}: core adt/parsing.rs apply_mcnk_subchunk and adt_tex.rs parse_tex0_mcnk do not consume this chunk; native terrain requests no file from it. No Retail-format completeness claim.",
            )
        if mcnks and len(self.graph.unresolved) == before_mcnk:
            status = "resolved" if needs_models or sounds else "not_needed"
            self.graph.resolve(
                "terrain_auxiliary_edges",
                fdid,
                status,
                f"MCNK: inspected all {mcnks} chunks; MCLY effects={sorted(effects)}, MCSE kits={sorted(sounds)}; other known subchunks hold inline geometry/alpha/shadow/placement indices (core adt_format)",
            )
        if "MH2O" in table:
            count, liquids = self.liquid_layers(table["MH2O"])
            before_liquid = len(self.graph.unresolved)
            for liquid_type, liquid_object in sorted(liquids):
                self.liquid(liquid_type, liquid_object, fdid)
            if len(self.graph.unresolved) == before_liquid:
                self.graph.resolve(
                    "terrain_auxiliary_edges",
                    fdid,
                    "resolved" if count else "not_needed",
                    f"MH2O: {count} instances, type/object pairs={sorted(liquids)}; texture identities via LiquidObject -> LiquidType -> LiquidTypeXTexture and shader globals (core liquid_data.rs); zero FDIDs are procedural",
                )
        if "MTXF" in table:
            integers(table["MTXF"])
            self.graph.resolve(
                "terrain_texture_flags",
                fdid,
                "not_needed",
                "MTXF u32s are flags/scales, not texture FDIDs; MDID/MHID hold diffuse/height FDIDs (core adt_tex.rs)",
            )
