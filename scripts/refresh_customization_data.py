#!/usr/bin/env python3
"""Refresh the character-customization CSVs in data/ for build 12.1.0.69933.

  refresh_customization_data.py [--data DIR]

1. Back up the current CSVs and customization caches to
   <data>/pre-12x-customization-20260930/ (once; an existing backup is kept).
2. Export ChrCustomizationElement, ChrCustomizationMaterial,
   ChrCustomizationSkinnedModel, ChrModelTextureLayer and ChrModelMaterial from the local CASC
   install (casc-local + scripts/export_db2_csv.py).
3. Fetch TextureFileData for 12.1.0.69933 from wago.tools as a build-pinned CSV
   (the game-server scripts/db2.py precedent). The local install has no
   TextureFileData.db2: its encoding key 83eb4cdc8845756de5748773dec63407 is in no
   Data/data/*.idx bucket file, so casc-local cannot read it. Textures still come
   from local CASC.

Then rebuild the caches the clients read with the root-crate importers:
  cargo run -j2 --bin customization_cache_import
  cargo run -j2 --bin char_texture_cache_import
"""

import argparse
import random
import shutil
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
EXPORTER = SCRIPT_DIR / "export_db2_csv.py"
CASC_LOCAL = Path("/syncthing/Sync/Projects/world-of-osso/asset-resolver/target/debug/casc-local")
BUILD = "12.1.0.69933"
BACKUP = "pre-12x-customization-20260930"
LOCAL_TABLES = {
    "ChrCustomizationElement": 3512765,
    "ChrCustomizationMaterial": 3459652,
    "ChrCustomizationSkinnedModel": 3460183,
    "ChrModelTextureLayer": 3548976,
    "ChrModelMaterial": 3566562,
    "ChrCustomizationReq": 3450453,
    "ChrCustomizationReqChoice": 3580359,
}
# TextureFileData.db2 is not in the local install. The localized tables' enUS copies
# have TACT-encrypted BLTE chunks (mode E) that the local readers do not decrypt.
WAGO_TABLES = [
    "TextureFileData",
    "ChrCustomizationChoice",
    "ChrCustomizationOption",
    "ChrCustomizationCategory",
    "ChrCustomizationGeoset",
    "CharHairGeosets",
]
BACKED_UP = [f"{table}.csv" for table in [*LOCAL_TABLES, *WAGO_TABLES]] + [
    "cache/customization-v4.sqlite",
    "cache/char_texture-v2.sqlite",
]
WAGO_URL = "https://wago.tools/db2/{table}/csv?build={build}"
MAX_ATTEMPTS = 6


def back_up(data):
    backup = data / BACKUP
    if backup.exists():
        print(f"keeping existing backup {backup}")
        return
    backup.mkdir()
    for name in BACKED_UP:
        source = data / name
        if source.exists():
            shutil.copy2(source, backup / Path(name).name)
    print(f"backed up to {backup}")


def export_local(data, table, fdid, work):
    subprocess.run([str(CASC_LOCAL), str(fdid), "-o", str(work)], cwd=data.parent, check=True,
                   capture_output=True)
    db2 = next(work.glob(f"{fdid}.*"))
    subprocess.run([sys.executable, str(EXPORTER), table, str(db2), str(data / f"{table}.csv")], check=True)


def is_transient(error):
    if isinstance(error, urllib.error.HTTPError):
        return error.code == 429 or error.code >= 500
    return isinstance(error, (urllib.error.URLError, TimeoutError, ConnectionError))


def retry_delay(attempt, error):
    retry_after = error.headers.get("Retry-After") if isinstance(error, urllib.error.HTTPError) else None
    if retry_after and retry_after.isdigit():
        return float(retry_after)
    return min(60.0, 2 ** attempt) * random.uniform(0.5, 1.0)


def fetch_wago(data, table):
    url = WAGO_URL.format(table=table, build=BUILD)
    for attempt in range(1, MAX_ATTEMPTS + 1):
        try:
            # wago.tools answers 403 to urllib's default User-Agent.
            request = urllib.request.Request(url, headers={"User-Agent": "world-of-osso-db2/1"})
            with urllib.request.urlopen(request, timeout=120) as response:
                if response.headers.get_content_type() != "text/csv":
                    raise RuntimeError(f"{url}: expected text/csv, got {response.headers.get_content_type()}")
                body = response.read()
            break
        except Exception as error:  # noqa: BLE001 - classified below
            if not is_transient(error) or attempt == MAX_ATTEMPTS:
                raise
            delay = retry_delay(attempt, error)
            print(f"  retry {attempt}/{MAX_ATTEMPTS - 1} in {delay:.1f}s: {error}", file=sys.stderr)
            time.sleep(delay)
    target = data / f"{table}.csv"
    partial = target.with_suffix(".csv.part")
    partial.write_bytes(body)
    partial.replace(target)
    print(f"{table}: {body.count(b'\n') - 1} rows from {url}")


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--data", type=Path, default=SCRIPT_DIR.parent / "data")
    data = parser.parse_args().data.resolve()
    back_up(data)
    with tempfile.TemporaryDirectory() as work:
        for table, fdid in LOCAL_TABLES.items():
            export_local(data, table, fdid, Path(work))
    for table in WAGO_TABLES:
        fetch_wago(data, table)


if __name__ == "__main__":
    main()
