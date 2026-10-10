#!/usr/bin/env python3
"""Read extracted bytes only. Never initialize CASC, extract, or access an install.

Binary layouts follow godot/core/src/asset/{adt,m2,wmo}_format. The manifest
is an audit, not a release certificate: absent bytes stop expansion explicitly;
legacy filenames and metadata build labels do NOT authenticate actual builds.
"""
import argparse
from collections import Counter, deque
import hashlib
import json
from pathlib import Path
import struct

EXPANDABLE = {"adt", "m2", "wmo", "skel", "wdt"}
DIRECTORIES = {"adt": "terrain", "wdt": "terrain", "wdl": "terrain", "blp": "textures", "ogg": "sounds", "mp3": "sounds", "wav": "sounds"}


def digest(path):
    h = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            h.update(block)
    return h.hexdigest()


def normalized(path):
    return path.replace("\\", "/").lower().replace(".mdx", ".m2")


def chunks(data, reverse=False):
    offset = 0
    while offset < len(data):
        if offset + 8 > len(data):
            raise ValueError(f"truncated chunk header at {offset}")
        tag = data[offset:offset + 4]
        size = struct.unpack_from("<I", data, offset + 4)[0]
        end = offset + 8 + size
        if end > len(data):
            raise ValueError(f"truncated {tag!r} at {offset}")
        yield (tag[::-1] if reverse else tag).decode("ascii"), data[offset + 8:end]
        offset = end


def records(data, stride):
    if len(data) % stride:
        raise ValueError(f"partial {stride}-byte record")
    return [data[i:i + stride] for i in range(0, len(data), stride)]


def u32(data, offset=0):
    return struct.unpack_from("<I", data, offset)[0]


def integers(data):
    return [u32(record) for record in records(data, 4)]


def string_at(data, offset):
    if offset >= len(data):
        raise ValueError(f"string offset {offset} outside {len(data)} bytes")
    end = data.find(b"\0", offset)
    if end < 0:
        raise ValueError("unterminated dependency path")
    return data[offset:end].decode("utf-8")


def read_listfile(path):
    result = {}
    with path.open(encoding="utf-8") as stream:
        for line in stream:
            fdid, logical = line.rstrip("\n").split(";", 1)
            result[int(fdid)] = normalized(logical)
    return result


class Closure:
    def __init__(self, data, paths, product, metadata_build):
        self.data = Path(data)
        self.paths = paths
        self.by_path = {}
        for fdid, path in paths.items():
            self.by_path.setdefault(normalized(path), []).append(fdid)
        self.product = product
        self.metadata_build = metadata_build
        self.assets = {}
        self.queue = deque()
        self.unresolved = set()
        self.inputs = {}
        self.seeds = {}

    def issue(self, code, reason, fdid=None):
        self.unresolved.add((code, reason, fdid))

    def input_file(self, path):
        relative = str(path.relative_to(self.data))
        if relative not in self.inputs:
            self.inputs[relative] = {"path": relative, "size": path.stat().st_size, "sha256": digest(path)}

    def add(self, fdid, kind, reason, parent=None, alias=None):
        if not fdid:
            return
        key = (int(fdid), kind)
        if key not in self.assets:
            self.assets[key] = {"fdid": int(fdid), "type": kind,
                                "logical_path": self.paths.get(int(fdid)),
                                "source_product": None, "requested_product": self.product,
                                "metadata_build": self.metadata_build,
                                "actual_build": None, "identity_status": "unverified",
                                "edges": set(), "locations": set()}
            self.queue.append(key)
        asset = self.assets[key]
        asset["edges"].add((parent, reason))
        asset["locations"].add(f"{DIRECTORIES.get(kind, 'models')}/{fdid}.{kind}")
        if alias and alias not in asset["locations"]:
            asset["locations"].add(alias)
            if asset.get("present") is False and kind in EXPANDABLE and (self.data / alias).is_file():
                self.queue.append(key)

    def named(self, logical, kind, reason, parent):
        ids = self.by_path.get(normalized(logical), [])
        if len(ids) != 1:
            self.issue("unmapped_path" if not ids else "ambiguous_path", f"{reason}: {logical}", parent)
            return
        self.add(ids[0], kind, reason, parent)

    def inspect(self, asset):
        found = []
        for location in sorted(asset["locations"]):
            path = self.data / location
            if path.is_file():
                found.append({"path": location, "size": path.stat().st_size, "sha256": digest(path)})
        asset["present"] = bool(found)
        asset["present_files"] = found
        asset["size"] = found[0]["size"] if found else None
        asset["sha256"] = found[0]["sha256"] if found else None
        if len({file["sha256"] for file in found}) > 1:
            self.issue("conflicting_local_bytes", f"{asset['type']} aliases differ", asset["fdid"])
            return None
        return self.data / found[0]["path"] if found else None

    def run(self):
        while self.queue:
            key = self.queue.popleft()
            asset = self.assets[key]
            path = self.inspect(asset)
            if asset["type"] not in EXPANDABLE:
                continue
            missing = ("missing_dependency_bytes", f"cannot expand {asset['type']}", asset["fdid"])
            if not path:
                if not asset["present"]:
                    self.unresolved.add(missing)
                continue
            self.unresolved.discard(missing)
            try:
                self.expand(asset, path.read_bytes())
            except (ValueError, struct.error, UnicodeError) as error:
                self.issue("parse_error", f"{asset['type']}: {error}", asset["fdid"])
        assets = []
        for key in sorted(self.assets):
            asset = self.assets[key]
            # An edge discovered after expansion can supply a runtime skin/skel alias.
            self.inspect(asset)
            row = dict(asset)
            row["locations"] = sorted(asset["locations"])
            row["edges"] = [{"parent_fdid": p, "reason": r} for p, r in sorted(asset["edges"], key=lambda edge: (edge[0] or 0, edge[1]))]
            assets.append(row)
        issues = [{"code": code, "reason": reason, "fdid": fdid}
                  for code, reason, fdid in sorted(self.unresolved, key=lambda issue: (issue[0], issue[2] or 0, issue[1]))]
        return {"schema_version": 1, "tool_sha256": digest(Path(__file__)), "seeds": self.seeds,
                "inputs": sorted(self.inputs.values(), key=lambda row: row["path"]),
                "assets": assets, "unresolved": issues,
                "summary": {"files": len(assets), "present": sum(a["present"] for a in assets),
                            "missing": sum(not a["present"] for a in assets), "unresolved": len(issues),
                            "unverified_identity": len(assets), "present_bytes": sum(a["size"] or 0 for a in assets),
                            "by_type": dict(sorted(Counter(a["type"] for a in assets).items()))}}

    def expand(self, asset, data):
        kind, fdid = asset["type"], asset["fdid"]
        if kind in {"m2", "skel"}:
            self.expand_model(fdid, kind, data)
        elif kind == "adt":
            self.expand_adt(fdid, data)
        elif kind == "wmo":
            self.expand_wmo(asset, data)
        elif kind == "wdt":
            for tag, payload in chunks(data, True):
                if tag == "MAID":
                    for i, row in enumerate(records(payload, 32)):
                        for column, value in enumerate(integers(row)):
                            kinds = ["adt", "adt", "adt", "adt", "adt", "blp", "blp", "blp"]
                            self.add(value, kinds[column], f"WDT MAID tile {i % 64},{i // 64} field {column}", fdid)

    def expand_model(self, fdid, kind, data):
        raw = data if data[:4] == b"MD20" else None
        for tag, payload in ([] if raw is not None else chunks(data)):
            if tag == "MD21":
                raw = payload
            elif tag in {"TXID", "SFID", "SKID"}:
                child_kind = {"TXID": "blp", "SFID": "skin", "SKID": "skel"}[tag]
                for i, value in enumerate(integers(payload)):
                    alias = f"models/{fdid}{i:02}.skin" if tag == "SFID" else None
                    if tag == "SKID":
                        alias = f"models/{fdid}.skel"
                    self.add(value, child_kind, f"{tag}[{i}]", fdid, alias)
            elif tag == "AFID":
                for row in records(payload, 8):
                    anim, variation, value = struct.unpack("<HHI", row)
                    self.add(value, "anim", f"AFID animation {anim}/{variation}", fdid)
            elif tag == "SKPD":
                # SKPD is four padding u32s followed by parent skeleton FDID.
                if len(payload) != 16:
                    raise ValueError("SKPD requires 16 bytes")
                self.add(u32(payload, 8), "skel", "SKPD parent skeleton", fdid)
            elif tag in {"BFID", "PFID"}:
                self.issue("unsupported_model_edge", tag, fdid)
        if kind == "m2":
            if raw is None or raw[:4] != b"MD20":
                raise ValueError("no MD20/MD21 model header")
            count, offset = struct.unpack_from("<II", raw, 0x50)
            if offset + count * 16 > len(raw):
                raise ValueError("M2 texture array out of bounds")
            for i in range(count):
                texture_type, flags, length, name_offset = struct.unpack_from("<IIII", raw, offset + i * 16)
                if length:
                    self.named(string_at(raw, name_offset), "blp", f"M2 named texture {i}", fdid)
            # TXID references cover particle/ribbon textures too. Replacement types
            # are supplied by display/customization/item seeds, not guessed here.
            for label, header in [("ribbon", 0x120), ("particle", 0x128)]:
                if len(raw) >= header + 8 and u32(raw, header):
                    self.issue("emitter_auxiliary_edges", f"{label}: embedded model/recursive emitter audit required", fdid)

    def expand_adt(self, fdid, data):
        table = dict(chunks(data, True))
        for tag in ["MDID", "MHID"]:
            for value in integers(table.get(tag, b"")):
                self.add(value, "blp", tag, fdid)
        for path in table.get("MTEX", b"").split(b"\0"):
            if path:
                self.named(path.decode(), "blp", "ADT MTEX", fdid)
        for tag, stride, flag_offset, mask, names, indices, kind in [
            ("MDDF", 36, 34, 0x40, "MMDX", "MMID", "m2"),
            ("MODF", 64, 56, 0x8, "MWMO", "MWID", "wmo")]:
            offsets = integers(table.get(indices, b""))
            for i, row in enumerate(records(table.get(tag, b""), stride)):
                index = u32(row)
                flags = struct.unpack_from("<H", row, flag_offset)[0]
                if flags & mask:
                    self.add(index, kind, f"ADT {tag}[{i}]", fdid)
                elif index < len(offsets):
                    self.named(string_at(table.get(names, b""), offsets[index]), kind, f"ADT {tag}[{i}]", fdid)
                else:
                    self.issue("invalid_name_index", f"{tag}[{i}] index {index}", fdid)
        for tag in ["MH2O", "MCNK"]:
            if tag in table:
                self.issue("terrain_auxiliary_edges", f"{tag}: liquid/ground-effect metadata not enumerated", fdid)

    def expand_wmo(self, asset, data):
        fdid = asset["fdid"]
        table = dict(chunks(data, True))
        groups = integers(table.get("GFID", b""))
        for i, value in enumerate(groups):
            self.add(value, "wmo", f"WMO GFID[{i}] (all LODs)", fdid)
        if "MOHD" in table and not groups:
            n_groups = u32(table["MOHD"], 4)
            logical = asset["logical_path"]
            for i in range(n_groups):
                if logical:
                    self.named(logical[:-4] + f"_{i:03}.wmo", "wmo", f"WMO group {i}", fdid)
                else:
                    self.issue("unnamed_wmo_group", f"group {i}", fdid)
        names = table.get("MOTX", b"")
        for i, row in enumerate(records(table.get("MOMT", b""), 64)):
            shader = u32(row, 4)
            extra = {19: 3, 20: 6}.get(shader, 0)
            for offset in [12, 24, 36] + [40 + i * 4 for i in range(extra)]:
                value = u32(row, offset)
                if not value:
                    continue
                if names:
                    self.named(string_at(names, value), "blp", f"WMO MOMT[{i}] field {offset}", fdid)
                else:
                    self.add(value, "blp", f"WMO MOMT[{i}] field {offset}", fdid)
            if u32(row, 4) > 23:
                self.issue("wmo_extended_material", f"MOMT[{i}] shader {u32(row, 4)} auxiliary slots", fdid)
        if "MODI" in table:
            for i, value in enumerate(integers(table["MODI"])):
                self.add(value, "m2", f"WMO MODI[{i}] (all doodad sets)", fdid)
        else:
            for i, row in enumerate(records(table.get("MODD", b""), 40)):
                self.named(string_at(table.get("MODN", b""), u32(row) & 0xffffff), "m2", f"WMO MODD[{i}]", fdid)
        if "MOGP" in table:
            # Group's nested chunks start after the 68-byte MOGP header.
            nested = dict(chunks(table["MOGP"][68:], True))
            if "MLIQ" in nested:
                self.issue("wmo_liquid_edges", "MLIQ liquid material metadata not enumerated", fdid)


def estimate_catalog(data, paths):
    """Broad listfile census, NOT authenticated/supported-content coverage.

    Missing bytes use the mean of present numeric-FDID files of that class.
    Local inventories may contain unrelated products/builds and skin aliases.
    No size information is invented for classes with no local sample.
    """
    classes = {}
    extensions = {"adt", "wdt", "wdl", "m2", "wmo", "skin", "skel", "anim", "blp", "ogg", "mp3"}
    for kind in sorted(extensions):
        ids = {fdid for fdid, path in paths.items() if path.endswith("." + kind)}
        roots = [data / DIRECTORIES.get(kind, "models")]
        if kind in {"ogg", "mp3"}:
            roots = [data / "sounds", data / "music"]
        inventory = [p for root in roots if root.is_dir() for p in root.rglob("*." + kind) if p.is_file()]
        local_ids = {int(p.stem) for p in inventory if p.stem.isdecimal()} & ids
        local_bytes = sum(p.stat().st_size for p in inventory)
        matched_bytes = sum(p.stat().st_size for p in inventory if p.stem.isdecimal() and int(p.stem) in ids)
        missing = len(ids - local_ids)
        mean = matched_bytes // len(local_ids) if local_ids else None
        classes[kind] = {"listfile_files": len(ids), "local_files": len(inventory), "local_bytes": local_bytes,
                         "legacy_numeric_id_matches": len(local_ids), "missing_file_estimate": missing,
                         "sample_mean_bytes": mean,
                         "missing_bytes_estimate": missing * mean if mean is not None else None,
                         "total_bytes_estimate": len(ids) * mean if mean is not None else None}
    return {"method": "Unfiltered listfile candidate superset across products/builds; not release closure. "
                      "Missing/total byte estimates multiply count by local matched-file mean, which is biased. "
                      "No compression estimate; unknown sizes are null. Local inventory is exact disk bytes, not authenticated coverage.",
            "classes": classes}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--data", type=Path, required=True)
    parser.add_argument("--world-db", type=Path, required=True)
    parser.add_argument("--config", type=Path, default=Path(__file__).with_name("closure-northshire.json"))
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--estimate-output", type=Path)
    args = parser.parse_args()
    from closure_seeds import seed_catalogs
    config = json.loads(args.config.read_text())
    graph = Closure(args.data, read_listfile(args.data / "community-listfile.csv"),
                    config["product"], config["metadata_build"])
    graph.input_file(args.data / "community-listfile.csv")
    if args.estimate_output:
        estimate = estimate_catalog(args.data, graph.paths)
        args.estimate_output.parent.mkdir(parents=True, exist_ok=True)
        args.estimate_output.write_text(json.dumps(estimate, indent=2, sort_keys=True) + "\n")
    seed_catalogs(graph, args.world_db, config)
    result = graph.run()
    result["config_sha256"] = digest(args.config)
    result["seed_tool_sha256"] = digest(Path(__file__).with_name("closure_seeds.py"))
    result["spell_seed_tool_sha256"] = digest(Path(__file__).with_name("closure_spell_seeds.py"))
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")
    print(json.dumps(result["summary"], sort_keys=True))
    # Audit output always written; unknown identity is deliberately not a certificate.
    return 1 if result["summary"]["missing"] or result["summary"]["unresolved"] or result["summary"]["unverified_identity"] else 0


if __name__ == "__main__":
    raise SystemExit(main())
