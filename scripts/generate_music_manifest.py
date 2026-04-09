#!/usr/bin/env python3

import csv
import json
import os
from collections import defaultdict
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
DATA_DIR = Path(__file__).resolve().parents[1] / "data"
if "GAME_ENGINE_SHARED_DATA_DIR" in os.environ:
    DATA_DIR = Path(os.environ["GAME_ENGINE_SHARED_DATA_DIR"])
MUSIC_DIR = DATA_DIR / "music"
LISTFILE = DATA_DIR / "community-listfile.csv"
SOUND_KIT_ENTRY = DATA_DIR / "SoundKitEntry.csv"
ZONE_MUSIC = DATA_DIR / "ZoneMusic.csv"
AREA_TABLE = DATA_DIR / "AreaTable.csv"

MANIFEST_CSV = DATA_DIR / "music_manifest.csv"
ZONE_LINKS_CSV = DATA_DIR / "music_zone_links.csv"
ZONE_INDEX_JSON = DATA_DIR / "music_zone_index.json"


def load_listfile():
    rows = []
    path_by_fdid = {}
    with LISTFILE.open(newline="") as handle:
        reader = csv.reader(handle, delimiter=";")
        for row in reader:
            if len(row) != 2:
                continue
            fdid, wow_path = row
            lower = wow_path.lower()
            if not lower.startswith("sound/music/"):
                continue
            if not (lower.endswith(".mp3") or lower.endswith(".ogg")):
                continue
            rows.append((fdid, wow_path))
            path_by_fdid[fdid] = wow_path
    return rows, path_by_fdid


def load_sound_kit_entries():
    kits_by_fdid = defaultdict(set)
    fdids_by_kit = defaultdict(set)
    with SOUND_KIT_ENTRY.open(newline="") as handle:
        reader = csv.DictReader(handle)
        for row in reader:
            fdid = row["FileDataID"]
            sound_kit_id = row["SoundKitID"]
            kits_by_fdid[fdid].add(sound_kit_id)
            fdids_by_kit[sound_kit_id].add(fdid)
    return kits_by_fdid, fdids_by_kit


def load_zone_music(fdids_by_kit):
    zone_music_rows = {}
    zone_music_by_fdid = defaultdict(set)
    with ZONE_MUSIC.open(newline="") as handle:
        reader = csv.DictReader(handle)
        for row in reader:
            zone_music_id = row["ID"]
            zone_music_rows[zone_music_id] = row
            for key in ("Sounds_0", "Sounds_1"):
                sound_kit_id = row[key]
                if sound_kit_id and sound_kit_id != "0":
                    for fdid in fdids_by_kit.get(sound_kit_id, ()):
                        zone_music_by_fdid[fdid].add(zone_music_id)
    return zone_music_rows, zone_music_by_fdid


def load_areas():
    areas_by_zone_music = defaultdict(list)
    with AREA_TABLE.open(newline="") as handle:
        reader = csv.DictReader(handle)
        for row in reader:
            zone_music_id = row["ZoneMusic"]
            if zone_music_id and zone_music_id != "0":
                areas_by_zone_music[zone_music_id].append(row)
    return areas_by_zone_music


def infer_path_context(wow_path):
    parts = wow_path.split("/")
    if len(parts) <= 3:
        return "", ""
    relative_dir = "/".join(parts[2:-1])
    hint = parts[3] if len(parts) > 4 else parts[2]
    return relative_dir, hint


def unique_sorted(items):
    return sorted(set(item for item in items if item))


def display_path(path: Path) -> str:
    try:
        return str(path.resolve().relative_to(ROOT.resolve()))
    except ValueError:
        return str(path)


def build_manifest():
    listfile_rows, path_by_fdid = load_listfile()
    kits_by_fdid, fdids_by_kit = load_sound_kit_entries()
    zone_music_rows, zone_music_by_fdid = load_zone_music(fdids_by_kit)
    areas_by_zone_music = load_areas()

    manifest_rows = []
    zone_link_rows = []
    zone_index = defaultdict(lambda: {"zone_music_sets": {}, "tracks": {}})

    for fdid, wow_path in listfile_rows:
        ext = Path(wow_path).suffix.lower()
        extracted_path = MUSIC_DIR / f"{fdid}{ext}"
        extracted = extracted_path.exists()
        path_category, path_hint = infer_path_context(wow_path)

        sound_kit_ids = unique_sorted(kits_by_fdid.get(fdid, ()))
        zone_music_ids = unique_sorted(zone_music_by_fdid.get(fdid, ()))
        zone_music_names = []
        area_ids = []
        area_names = []
        zone_display_names = []
        zone_internal_names = []

        for zone_music_id in zone_music_ids:
            row = zone_music_rows.get(zone_music_id)
            if row:
                zone_music_names.append(row["SetName"])
            for area in areas_by_zone_music.get(zone_music_id, ()):
                area_ids.append(area["ID"])
                area_names.append(area["AreaName_lang"])
                zone_display_names.append(area["AreaName_lang"])
                zone_internal_names.append(area["ZoneName"])
                zone_link_rows.append(
                    {
                        "fdid": fdid,
                        "ext": ext.lstrip("."),
                        "extracted": "1" if extracted else "0",
                        "wow_path": wow_path,
                        "path_category": path_category,
                        "path_hint": path_hint,
                        "sound_kit_ids": "|".join(sound_kit_ids),
                        "zone_music_id": zone_music_id,
                        "zone_music_set_name": row["SetName"] if row else "",
                        "area_id": area["ID"],
                        "area_name": area["AreaName_lang"],
                        "zone_internal_name": area["ZoneName"],
                    }
                )

        association_method = "exact_zone_music" if zone_music_ids and area_ids else "path_only"

        manifest_rows.append(
            {
                "fdid": fdid,
                "ext": ext.lstrip("."),
                "extracted": "1" if extracted else "0",
                "extracted_path": display_path(extracted_path) if extracted else "",
                "wow_path": wow_path,
                "path_category": path_category,
                "path_hint": path_hint,
                "association_method": association_method,
                "sound_kit_ids": "|".join(sound_kit_ids),
                "zone_music_ids": "|".join(zone_music_ids),
                "zone_music_set_names": "|".join(unique_sorted(zone_music_names)),
                "area_ids": "|".join(unique_sorted(area_ids)),
                "area_names": "|".join(unique_sorted(area_names)),
                "zone_display_names": "|".join(unique_sorted(zone_display_names)),
                "zone_internal_names": "|".join(unique_sorted(zone_internal_names)),
            }
        )

        if extracted:
            for zone_name in unique_sorted(zone_display_names):
                entry = zone_index[zone_name]
                entry["tracks"][fdid] = {
                    "wow_path": wow_path,
                    "path_category": path_category,
                    "area_names": unique_sorted(area_names),
                    "zone_internal_names": unique_sorted(zone_internal_names),
                }
                for zone_music_id in zone_music_ids:
                    zone_music_name = zone_music_rows.get(zone_music_id, {}).get("SetName", "")
                    if zone_music_name:
                        entry["zone_music_sets"][zone_music_id] = zone_music_name

    manifest_rows.sort(key=lambda row: int(row["fdid"]))

    with MANIFEST_CSV.open("w", newline="") as handle:
        fieldnames = [
            "fdid",
            "ext",
            "extracted",
            "extracted_path",
            "wow_path",
            "path_category",
            "path_hint",
            "association_method",
            "sound_kit_ids",
            "zone_music_ids",
            "zone_music_set_names",
            "area_ids",
            "area_names",
            "zone_display_names",
            "zone_internal_names",
        ]
        writer = csv.DictWriter(handle, fieldnames=fieldnames)
        writer.writeheader()
        writer.writerows(manifest_rows)

    zone_link_rows.sort(
        key=lambda row: (
            row["area_name"],
            int(row["fdid"]),
            int(row["zone_music_id"]),
            int(row["area_id"]),
        )
    )
    with ZONE_LINKS_CSV.open("w", newline="") as handle:
        fieldnames = [
            "fdid",
            "ext",
            "extracted",
            "wow_path",
            "path_category",
            "path_hint",
            "sound_kit_ids",
            "zone_music_id",
            "zone_music_set_name",
            "area_id",
            "area_name",
            "zone_internal_name",
        ]
        writer = csv.DictWriter(handle, fieldnames=fieldnames)
        writer.writeheader()
        writer.writerows(zone_link_rows)

    summary = {
        "expected_audio_entries": len(manifest_rows),
        "extracted_audio_entries": sum(1 for row in manifest_rows if row["extracted"] == "1"),
        "exact_zone_music_matches": sum(
            1
            for row in manifest_rows
            if row["extracted"] == "1" and row["association_method"] == "exact_zone_music"
        ),
        "path_only_matches": sum(
            1 for row in manifest_rows if row["extracted"] == "1" and row["association_method"] == "path_only"
        ),
        "zones_with_exact_matches": len(zone_index),
        "zones": {
            zone_name: {
                "zone_music_sets": dict(sorted(payload["zone_music_sets"].items(), key=lambda item: int(item[0]))),
                "tracks": dict(sorted(payload["tracks"].items(), key=lambda item: int(item[0]))),
            }
            for zone_name, payload in sorted(zone_index.items())
        },
    }

    with ZONE_INDEX_JSON.open("w") as handle:
        json.dump(summary, handle, indent=2, sort_keys=False)

    missing = [row["fdid"] for row in manifest_rows if row["extracted"] == "0"]
    print(f"wrote {display_path(MANIFEST_CSV)}")
    print(f"wrote {display_path(ZONE_LINKS_CSV)}")
    print(f"wrote {display_path(ZONE_INDEX_JSON)}")
    print(f"expected_audio_entries={summary['expected_audio_entries']}")
    print(f"extracted_audio_entries={summary['extracted_audio_entries']}")
    print(f"exact_zone_music_matches={summary['exact_zone_music_matches']}")
    print(f"path_only_matches={summary['path_only_matches']}")
    print(f"zones_with_exact_matches={summary['zones_with_exact_matches']}")
    print(f"missing_audio_entries={len(missing)}")


if __name__ == "__main__":
    build_manifest()
