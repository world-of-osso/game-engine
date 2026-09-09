"""Behavioral fixtures for the bounded local NPC importer (stdlib only)."""

import importlib.util
import contextlib
import io
import gc
import warnings
import json
import sqlite3
import struct
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).with_name("import_npc_appearance.py")


def load_importer():
    if not SCRIPT.exists():
        return None
    spec = importlib.util.spec_from_file_location("npc_importer", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def fixture(layout=0x4D9FE25C, related=False, copies=(), encrypted=False):
    """Two concrete records, palette lookup, common override, signed packing."""
    if related:
        fields = [(0, 8, 0, 0, 0, 0, 0), (8, 8, 0, 1, 8, 8, 0)]
        records = bytes([3, 9, 4, 10])
        palette = common = b""
        ids = struct.pack("<2I", 101, 102)
        relations = struct.pack("<7I", 2, 17, 18, 17, 0, 18, 1)
        size, flags = 2, 4
    else:
        fields = [
            (0, 16, 0, 5, 0, 16, 0),
            (16, 1, 8, 3, 16, 1, 0),
            (17, 2, 0, 5, 17, 2, 0),
            (19, 4, 0, 1, 19, 4, 0),
            (23, 0, 8, 2, 7, 0, 0),
            (23, 16, 0, 5, 23, 16, 0),
            (39, 16, 0, 5, 39, 16, 0),
        ]
        records = b"".join(
            (rid | race << 16 | sex << 17 | 2 << 19 | 31 << 23 | 32 << 39).to_bytes(
                7, "little"
            )
            for rid, race, sex in [(17, 0, 0), (18, 1, 3)]
        )
        palette = struct.pack("<2I", 1, 4)
        common = struct.pack("<2I", 18, 42)
        ids = relations = b""
        size, flags = 7, 0
    n = len(fields)
    start = 204 + 40 + n * 4 + n * 24 + len(palette) + len(common)
    header = bytearray(204)
    header[:4] = b"WDC5"
    struct.pack_into("<I", header, 4, 5)
    struct.pack_into("<9I", header, 136, 2, n, size, 0, 0, layout, 17, 102, 0)
    struct.pack_into(
        "<HH7I",
        header,
        172,
        flags,
        0,
        n,
        0,
        int(related),
        n * 24,
        len(common),
        len(palette),
        1,
    )
    section = struct.pack(
        "<Q8I", int(encrypted), start, 2, 0, 0, len(ids), len(relations), 0, len(copies)
    )
    return (
        bytes(header)
        + section
        + bytes(n * 4)
        + b"".join(struct.pack("<HH5I", *f) for f in fields)
        + palette
        + common
        + records
        + ids
        + b"".join(struct.pack("<2I", *pair) for pair in copies)
        + relations
    )


class ImportTests(unittest.TestCase):
    def setUp(self):
        self.m = load_importer()
        self.assertIsNotNone(self.m, "NPC importer not implemented")

    def test_database_helpers_close_connections_without_resource_warnings(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "closed.sqlite"
            with warnings.catch_warnings(record=True) as observed:
                warnings.simplefilter("always", ResourceWarning)
                self.m.write_database(path, ([], [], [], [(42, 0)]))
                self.assertEqual(
                    self.m.read_sqlite(path, "SELECT display_id FROM display_coverage"),
                    [(42,)],
                )
                gc.collect()
            self.assertEqual(
                [
                    str(item.message)
                    for item in observed
                    if item.category is ResourceWarning
                ],
                [],
            )

    def test_inline_palette_common_signed_and_copy(self):
        rows = self.m.read_wdc5(fixture(copies=[(19, 18)]), "extra")
        self.assertEqual(
            rows,
            {
                17: ((17, 1, 0, 2, 7, 31, 32), None),
                18: ((18, 4, -1, 2, 42, 31, 32), None),
                19: ((19, 4, -1, 2, 7, 31, 32), None),
            },
        )

    def test_noninline_relations_and_copy(self):
        for kind, layout in [("option", 0x2F331C33), ("geoset", 0x5E539080)]:
            with self.subTest(kind=kind):
                self.assertEqual(
                    self.m.read_wdc5(fixture(layout, True, [(103, 101)]), kind),
                    {101: ((3, 9), 17), 102: ((4, 10), 18), 103: ((3, 9), 17)},
                )

    def test_decrypted_keyed_section_is_read(self):
        self.assertEqual(len(self.m.read_wdc5(fixture(encrypted=True), "extra")), 2)

    def test_rejects_unsupported_or_corrupt_inputs(self):
        cases = []
        b = bytearray(fixture())
        struct.pack_into("<I", b, 156, 1)
        cases.append((b, "layout"))
        b = bytearray(fixture())
        struct.pack_into("<I", b, 204 + 40 + 28 + 8, 4)
        cases.append((b, "storage"))
        b = bytearray(fixture())
        struct.pack_into("<H", b, 172, 1)
        cases.append((b, "flags"))
        b = bytearray(fixture(encrypted=True))
        b[-14:] = bytes(14)
        cases.append((b, "encrypted"))
        cases.append((fixture()[:-1], "truncated"))
        cases.append((fixture(copies=[(20, 999)]), "copy"))
        for data, message in cases:
            with (
                self.subTest(message=message),
                self.assertRaisesRegex(ValueError, message),
            ):
                self.m.read_wdc5(data, "extra")

    def test_join_selects_authored_hd_sd_and_preserves_regular(self):
        displays = {13035: 17, 13036: 17, 130617: 18, 99: 0}
        extra = {
            17: ((17, 1, 0, 2, 0, 31, 32), None),
            18: ((18, 4, 1, 3, 0, 33, 34), None),
        }
        options = {1: ((10, 100), 17), 2: ((11, 101), 17), 3: ((12, 102), 18)}
        geosets = {1: ((3, 2), 13035), 2: ((4, 1), 130617), 3: ((1, 1), 99)}
        paths = {
            13035: "character/human/male/humanmale_hd.m2",
            13036: "character/human/male/humanmale.m2",
            130617: "character/nightelf/female/nightelffemale_hd.m2",
        }
        result = self.m.join_appearances(
            displays, extra, options, geosets, paths, {31: 501, 32: 502, 34: 504}
        )
        self.assertEqual(
            result,
            (
                [(13035, 1, 0, 2, 502), (13036, 1, 0, 2, 501), (130617, 4, 1, 3, 504)],
                [(13035, 100), (13035, 101), (13036, 100), (13036, 101), (130617, 102)],
                [(99, 1, 1), (13035, 3, 2), (130617, 4, 1)],
                [(99, 0), (13035, 1), (13036, 1), (130617, 1)],
            ),
        )
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / "appearance.sqlite"
            self.m.write_database(out, result)
            first = out.read_bytes()
            with contextlib.closing(sqlite3.connect(out)) as conn, conn:
                self.assertEqual(
                    conn.execute(
                        "select * from appearances order by display_id"
                    ).fetchall(),
                    result[0],
                )
                self.assertEqual(
                    conn.execute(
                        "select * from choices order by display_id, choice_id"
                    ).fetchall(),
                    result[1],
                )
                self.assertEqual(
                    conn.execute(
                        "select * from geosets order by display_id, geoset_index"
                    ).fetchall(),
                    result[2],
                )
                self.assertEqual(
                    conn.execute(
                        "select * from display_coverage order by display_id"
                    ).fetchall(),
                    result[3],
                )
            out.unlink()
            self.m.write_database(out, result)
            self.assertEqual(out.read_bytes(), first)
            with self.assertRaisesRegex(ValueError, "exists"):
                self.m.write_database(out, result)

    def test_cli_imports_local_files_and_reports_rows(self):
        self.assertTrue(hasattr(self.m, "main"), "CLI importer not implemented")
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            for filename, data in [
                ("1264997.db2", fixture()),
                ("3692043.db2", fixture(0x2F331C33, True)),
                ("1720141.db2", fixture(0x5E539080, True)),
            ]:
                (root / filename).write_bytes(data)
            (root / "CreatureDisplayInfo.csv").write_text(
                "ID,ExtendedDisplayInfoID\n17,17\n18,0\n19,0\n"
            )
            (root / "community-listfile.csv").write_text(
                "100;character/human/male/humanmale_hd.m2\n"
            )
            (root / "TextureFileData.csv").write_text(
                "FileDataID,UsageType,MaterialResourcesID\n500,0,31\n501,0,32\n"
            )
            with (
                contextlib.closing(sqlite3.connect(root / "models.sqlite")) as conn,
                conn,
            ):
                conn.execute(
                    "create table creature_displays (display_id integer, model_fdid integer)"
                )
                conn.execute("insert into creature_displays values (17, 100)")
            args = [
                "--db2-dir",
                str(root),
                "--data-dir",
                str(root),
                "--model-cache",
                str(root / "models.sqlite"),
                "--display-id",
                "17",
                "--display-id",
                "18",
                "--output",
                str(root / "result.sqlite"),
            ]
            output = io.StringIO()
            with contextlib.redirect_stdout(output):
                self.assertEqual(self.m.main(args), 0)
            report = json.loads(output.getvalue())
            self.assertEqual(
                report["counts"],
                {"appearances": 1, "choices": 1, "geosets": 2, "display_coverage": 2},
            )
            with (
                contextlib.closing(sqlite3.connect(root / "result.sqlite")) as conn,
                conn,
            ):
                self.assertEqual(
                    conn.execute("select * from appearances").fetchall(),
                    [(17, 1, 0, 2, 501)],
                )
                self.assertEqual(
                    conn.execute("select * from choices").fetchall(), [(17, 9)]
                )
                self.assertEqual(
                    conn.execute(
                        "select * from display_coverage order by display_id"
                    ).fetchall(),
                    [(17, 1), (18, 0)],
                )
                self.assertIsNone(
                    conn.execute(
                        "select requires_appearance from display_coverage where display_id=19"
                    ).fetchone()
                )
            with (
                contextlib.closing(sqlite3.connect(root / "outfits.sqlite")) as conn,
                conn,
            ):
                conn.execute(
                    "create table material_to_texture (material_resource_id integer, texture_fdid integer)"
                )
                conn.execute("insert into material_to_texture values (32, 501)")
            args[-1] = str(root / "cached.sqlite")
            with contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(
                    self.m.main(
                        args + ["--outfit-cache", str(root / "outfits.sqlite")]
                    ),
                    0,
                )
            self.assertEqual(
                (root / "result.sqlite").read_bytes(),
                (root / "cached.sqlite").read_bytes(),
            )
            (root / "TextureFileData.csv").write_text(
                "FileDataID,UsageType,MaterialResourcesID\n501,0,32\n502,0,32\n"
            )
            args[-1] = str(root / "failed.sqlite")
            with contextlib.redirect_stderr(io.StringIO()) as errors:
                self.assertEqual(self.m.main(args), 1)
            self.assertIn("ambiguous material", errors.getvalue())
            self.assertFalse((root / "failed.sqlite").exists())

    def test_zero_selected_hd_material_preserves_absence_without_sd_fallback(self):
        rows = self.m.join_appearances(
            {7: 17},
            {17: ((17, 1, 0, 2, 0, 31, 0), None)},
            {},
            {},
            {7: "character/human/male/humanmale_hd.m2"},
            {31: 501},
        )
        self.assertEqual(rows[0], [(7, 1, 0, 2, 0)])
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / "appearance.sqlite"
            self.m.write_database(out, rows)
            with contextlib.closing(sqlite3.connect(out)) as conn:
                self.assertEqual(
                    conn.execute(
                        "SELECT baked_texture_fdid FROM appearances WHERE display_id = 7"
                    ).fetchone(),
                    (0,),
                )

    def test_nonzero_selected_hd_material_still_requires_texture_mapping(self):
        with self.assertRaisesRegex(ValueError, "display 7: unresolved material 32"):
            self.m.join_appearances(
                {7: 17},
                {17: ((17, 1, 0, 2, 0, 31, 32), None)},
                {},
                {},
                {7: "character/human/male/humanmale_hd.m2"},
                {31: 501},
            )

    def test_missing_extra_model_material_fail(self):
        extra = {17: ((17, 1, 0, 2, 0, 31, 32), None)}
        for extras, paths, textures, error in [
            ({}, {}, {}, "Extra"),
            (extra, {}, {}, "model"),
            (extra, {7: "humanmale_hd.m2"}, {31: 10}, "material"),
        ]:
            with self.subTest(error=error), self.assertRaisesRegex(ValueError, error):
                self.m.join_appearances({7: 17}, extras, {}, {}, paths, textures)


if __name__ == "__main__":
    unittest.main()
