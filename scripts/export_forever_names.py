#!/usr/bin/env python3
"""Export pinned local Forever NameGen without extraction or other imports."""

import argparse
import hashlib
import json
import struct
import sys
from pathlib import Path

try:
    from scripts import import_forever_skyborne as importer
except ModuleNotFoundError:
    import import_forever_skyborne as importer

SOURCE_SHA256 = "3e9a6f3e8d0cd5d2976ee63f5b4fc10aa2960497efa3fe623249f533b34321ae"
DBD_SHA256 = "03b91a2a5d0d316f138c1aad9cf736256f5a84adcefba5cef4a851d838729393"


def export_names(data):
    fdid = importer.TABLES["NameGen"]
    source_root = data / f"forever-{importer.BUILD}"
    source = source_root / "skyborne-probe" / f"{fdid}.db2"
    definition = source_root / "definitions/NameGen.dbd"
    raw, dbd = source.read_bytes(), definition.read_bytes()
    for path, content, expected in (
        (source, raw, SOURCE_SHA256),
        (definition, dbd, DBD_SHA256),
    ):
        actual = hashlib.sha256(content).hexdigest()
        if actual != expected:
            raise ValueError(f"{path}: SHA256 {actual} != pinned {expected}")
    table_hash, layout = struct.unpack_from("<II", raw, 152)
    expected_layout, expected_columns = importer.export.TABLES["NameGen"]
    if raw[:4] != b"WDC5" or layout != expected_layout:
        raise ValueError(f"NameGen: expected WDC5 layout {expected_layout:08X}")
    columns, id_field, explicit = importer.parse_definition(dbd.decode(), layout)
    if columns != expected_columns:
        raise ValueError("NameGen DBD schema differs from maintained export registry")
    rows, dropped = importer.decode_rows(raw, layout, columns, id_field)
    if dropped:
        raise ValueError(f"NameGen: {dropped} encrypted rows dropped")
    encoded, missing = importer.encode_csv(b"", columns, rows, table="NameGen")
    if missing:
        raise ValueError(f"NameGen: missing columns {missing}")
    provenance = {
        "product": "wow_classic_beta",
        "build": importer.BUILD,
        "build_key": importer.BUILD_KEY,
        "fdid": fdid,
        "source_file": str(source),
        "sha256": SOURCE_SHA256,
        "TableHash": f"{table_hash:08X}",
        "LayoutHash": f"{layout:08X}",
        "dbd_file": str(definition),
        "dbd_sha256": DBD_SHA256,
        "explicit_build": explicit,
        "layout_match": "explicit-build" if explicit else "hash-match",
        "columns": [name for name, _ in columns],
        "rows_decoded": len(rows),
        "encrypted_rows_dropped": dropped,
        "csv_sha256": hashlib.sha256(encoded).hexdigest(),
    }
    output = data / "db2" / importer.BUILD
    output.mkdir(parents=True, exist_ok=True)
    (output / "NameGen.csv").write_bytes(encoded)
    (output / "NameGen.provenance.json").write_text(
        json.dumps(provenance, indent=2) + "\n"
    )
    return output, len(rows)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--data", type=Path, default=Path(__file__).resolve().parents[1] / "data"
    )
    args = parser.parse_args()
    try:
        output, count = export_names(args.data)
    except (OSError, ValueError, KeyError, struct.error) as error:
        print(f"NameGen export failed: {error}", file=sys.stderr)
        return 1
    print(f"NameGen: {count} rows; CSV and provenance in {output}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
