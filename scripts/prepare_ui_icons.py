"""Developer/depot asset preparation; never imported by the shipped client.

Enumerate all positive spell/trait icon metadata, not guessed learned builds, plus
source-local item icons and class/spec portraits. Runtime reads extracted BLP only.
"""
import argparse
import csv
import hashlib
import json
import os
from pathlib import Path
import subprocess

SPELL_BUILD = "12.1.0.69933"
MANIFEST = Path("cache/required-ui-icons.json")


def hash_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def icon_tables(data: Path) -> dict[Path, tuple[str, ...]]:
    directory = Path("db2") / SPELL_BUILD
    tables = {
        directory / "SpellMisc.csv": ("SpellIconFileDataID", "ActiveIconFileDataID"),
        directory / "TraitDefinition.csv": ("OverrideIcon",),
        directory / "ChrClasses.csv": ("IconFileDataID",),
        directory / "ChrSpecialization.csv": ("SpellIconFileID",),
    }
    for pattern, column in [
        ("Item.csv", "IconFileDataID"),
        ("ItemAppearance.csv", "DefaultIconFileDataID"),
        ("db2/*/Item.csv", "IconFileDataID"),
        ("db2/*/ItemAppearance.csv", "DefaultIconFileDataID"),
        ("db2/*/items/Item.csv", "IconFileDataID"),
        ("db2/*/items/ItemAppearance.csv", "DefaultIconFileDataID"),
    ]:
        for path in data.glob(pattern):
            tables[path.relative_to(data)] = (column,)
    return tables


def collect_required_icons(data: Path) -> dict:
    sources: dict[int, set[str]] = {}
    table_hashes = {}
    for relative, columns in sorted(icon_tables(data).items()):
        path = data / relative
        table_hashes[str(relative)] = hash_file(path)
        with path.open(newline="") as stream:
            reader = csv.DictReader(stream)
            absent = set(columns) - set(reader.fieldnames or [])
            if absent:
                raise ValueError(f"{path}: missing icon columns {sorted(absent)}")
            for row in reader:
                for column in columns:
                    fdid = int(row[column])
                    if fdid > 0:
                        sources.setdefault(fdid, set()).add(f"{relative}:{column}")
    return {
        "version": 1,
        "spell_db2_build": SPELL_BUILD,
        "scope": "All positive SpellMisc/trait/class/spec icons and source-local item icons; superset of reachable spellbook/talent icons.",
        "source_tables": table_hashes,
        "icons": [{"fdid": fdid, "sources": sorted(origin)} for fdid, origin in sorted(sources.items())],
    }


def missing_required_icons(manifest: dict, data: Path) -> list[int]:
    missing = []
    for entry in manifest["icons"]:
        fdid = entry["fdid"]
        path = data / "textures" / f"{fdid}.blp"
        if not path.is_file():
            missing.append(fdid)
            continue
        with path.open("rb") as source:
            if source.read(4) not in (b"BLP1", b"BLP2"):
                missing.append(fdid)
    return missing


def write_json_if_changed(path: Path, value: dict) -> None:
    encoded = json.dumps(value, indent=2) + "\n"
    path.parent.mkdir(parents=True, exist_ok=True)
    if not path.is_file() or path.read_text() != encoded:
        path.write_text(encoded)


def normalize_extracted_icon_names(data: Path, fdids: list[int]) -> None:
    # casc-local uses .dat when the local listfile has no name. FDID and BLP header
    # still identify the same authentic bytes; normalize their cache filename only.
    for fdid in fdids:
        source = data / "textures" / f"{fdid}.dat"
        destination = data / "textures" / f"{fdid}.blp"
        if source.is_file() and not destination.exists():
            with source.open("rb") as stream:
                is_blp = stream.read(4) in (b"BLP1", b"BLP2")
            if is_blp:
                source.rename(destination)


def record_icon_provenance(data: Path, manifest: dict, missing: list[int]) -> None:
    absent = set(missing)
    records = []
    for entry in manifest["icons"]:
        fdid = entry["fdid"]
        if fdid not in absent:
            path = data / "textures" / f"{fdid}.blp"
            records.append({"fdid": fdid, "sha256": hash_file(path), "bytes": path.stat().st_size})
    write_json_if_changed(data / "cache/ui-icon-provenance.json", {
        "status": "incomplete" if missing else "complete",
        "missing_fdids": missing,
        "extractor": "casc-local (local WoW archives only; developer build step)",
        "source_tables": manifest["source_tables"],
        "icons": records,
    })


def extract_missing_icons(root: Path, extractor: Path, missing: list[int], evidence: Path) -> int:
    if not extractor.is_file():
        raise ValueError(f"UI icon pre-extraction needs {extractor}; {len(missing)} required FDIDs absent: {missing[:32]}")
    environment = dict(os.environ, WOW_PRODUCT="wow")
    with (evidence / "extract.log").open("a") as log:
        log.write(f"\nPreparing {len(missing)} declared UI icon FDIDs from local CASC only\n")
        log.flush()
        result = subprocess.run([str(extractor), *map(str, missing), "-o", str(root / "data/textures")],
                                cwd=root, env=environment, stdout=log, stderr=subprocess.STDOUT)
    normalize_extracted_icon_names(root / "data", missing)
    return result.returncode


def prepare_ui_icons(root: Path, extractor: Path) -> dict:
    data = root / "data"
    manifest = collect_required_icons(data)
    write_json_if_changed(data / MANIFEST, manifest)
    missing = missing_required_icons(manifest, data)
    record_icon_provenance(data, manifest, missing)
    evidence = data / "diagnostics/ui-icon-preparation"
    evidence.mkdir(parents=True, exist_ok=True)
    if missing:
        status = extract_missing_icons(root, extractor, missing, evidence)
        missing = missing_required_icons(manifest, data)
        record_icon_provenance(data, manifest, missing)
        if status or missing:
            write_json_if_changed(evidence / "missing.json", {"fdids": missing, "extract_exit": status})
            raise ValueError(f"UI asset preparation failed: {len(missing)} required icons absent/invalid: {missing[:32]}; complete list/log: {evidence}")
    (evidence / "missing.json").unlink(missing_ok=True)
    print(f"Prepared {len(manifest['icons'])} required UI icon BLPs; manifest/provenance ship in data/cache/", flush=True)
    return manifest


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--extractor", type=Path)
    args = parser.parse_args()
    root = args.root.resolve()
    extractor = args.extractor or root.parent / "asset-resolver/target/debug/casc-local"
    try:
        prepare_ui_icons(root, extractor)
    except (OSError, ValueError) as error:
        parser.exit(1, f"{error}\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
