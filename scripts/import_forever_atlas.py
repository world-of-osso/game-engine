#!/usr/bin/env python3
"""Import WoW Forever's UI atlas tables and their set-1 (`c60`) textures into data/.

  import_forever_atlas.py [--data DIR]

1. Copy UiTextureAtlas, UiTextureAtlasElement, UiTextureAtlasMember and
   UiTextureAtlasElementSliceData (Wago DB2CSV exports of 1.60.1.69913, sha256-checked
   against the source provenance.json) to <data>/db2/1.60.1.69913/. The local
   wow_classic_beta 1.60.1.70205 install cannot provide them: its root encoding key
   fcae3917977c7fdf9f3864ed5bf96521 is in no Data/data/*.idx bucket file.
   UiTextureAtlas.UiTextureAtlasSetID is Forever-only: set 1 holds the `c60` re-skins of
   Retail atlas names.
2. Write scripts/forever-atlas-listfile.csv (`FDID;path`): every set-1 atlas texture the
   Forever 69913 listfile names. None is in data/community-listfile.csv.
3. Extract every set-1 atlas texture absent from <data>/textures/ from local CASC
   (casc-local, WOW_PRODUCT=wow_classic_beta) as <data>/textures/<fdid>.blp, and list
   the FDIDs that fail.
"""

import argparse
import csv
import hashlib
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
SOURCE = Path("/syncthing/Sync/Projects/wow/wow-ui-sim/data/db2/wowforever-1.60.1.69913")
CASC_LOCAL = Path("/syncthing/Sync/Projects/world-of-osso/asset-resolver/target/debug/casc-local")
BUILD = "1.60.1.69913"
TABLES = [
    "UiTextureAtlas.csv",
    "UiTextureAtlasElement.csv",
    "UiTextureAtlasMember.csv",
    "UiTextureAtlasElementSliceData.csv",
]
LISTFILE = SCRIPT_DIR / "forever-atlas-listfile.csv"


def copy_tables(out_dir):
    provenance = json.loads((SOURCE / "provenance.json").read_text())
    out_dir.mkdir(parents=True, exist_ok=True)
    for table in TABLES + ["listfile.csv"]:
        data = (SOURCE / table).read_bytes()
        digest = hashlib.sha256(data).hexdigest()
        if digest != provenance["sha256"][table]:
            sys.exit(f"{SOURCE / table}: sha256 {digest} does not match provenance.json")
        if table in TABLES:
            (out_dir / table).write_bytes(data)
    shutil.copyfile(SOURCE / "provenance.json", out_dir / "provenance.json")


def set_one_textures(db2_dir):
    with open(db2_dir / "UiTextureAtlas.csv", newline="") as handle:
        rows = csv.DictReader(handle)
        return sorted({int(row["FileDataID"]) for row in rows if row["UiTextureAtlasSetID"] == "1"})


def write_listfile(fdids):
    paths = {}
    for line in (SOURCE / "listfile.csv").read_text().splitlines():
        fdid, path = line.split(";", 1)
        paths[int(fdid)] = path
    named = [fdid for fdid in fdids if fdid in paths]
    LISTFILE.write_text("".join(f"{fdid};{paths[fdid]}\n" for fdid in named))
    return len(named)


def extract_textures(data, fdids):
    textures = data / "textures"
    staging = data / "cache" / "forever-atlas-extract"
    staging.mkdir(parents=True, exist_ok=True)
    missing = [fdid for fdid in fdids if not (textures / f"{fdid}.blp").is_file()]
    if missing:
        env = dict(os.environ, WOW_PRODUCT="wow_classic_beta")
        args = [str(CASC_LOCAL), *map(str, missing), "-o", str(staging)]
        subprocess.run(args, env=env, check=False)
    failed = []
    for fdid in missing:
        # casc-local names files by the community listfile, which lacks every c60 path.
        extracted = sorted(staging.glob(f"{fdid}.*"))
        if not extracted:
            failed.append(fdid)
            continue
        extracted[0].replace(textures / f"{fdid}.blp")
    return len(fdids) - len(missing), failed


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--data", type=Path, default=SCRIPT_DIR.parent / "data")
    args = parser.parse_args()
    db2_dir = args.data / "db2" / BUILD
    copy_tables(db2_dir)
    fdids = set_one_textures(db2_dir)
    named = write_listfile(fdids)
    present, failed = extract_textures(args.data, fdids)
    print(f"set-1 textures: {len(fdids)}, named {named}, already present {present}, "
          f"extracted {len(fdids) - present - len(failed)}, failed {len(failed)}")
    if failed:
        print("failed FDIDs: " + " ".join(map(str, failed)))
        sys.exit(1)


if __name__ == "__main__":
    main()
