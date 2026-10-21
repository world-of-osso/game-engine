"""Dynamic world game objects seed the same model identities as native rendering."""

from pathlib import Path
import sqlite3
import struct
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from asset_closure import Closure
import closure_seeds


def chunk(tag, payload):
    return tag.encode() + struct.pack("<I", len(payload)) + payload


class GameObjectRootsTests(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.data = Path(directory.name)
        (self.data / "models").mkdir()
        csv = self.data / "db2/fixture/GameObjectDisplayInfo.csv"
        csv.parent.mkdir(parents=True)
        csv.write_text(
            "ID,FileDataID\n2690,200010\n2700,200011\n2701,200012\n2702,200013\n"
        )
        raw = b"MD20" + bytes(124)
        (self.data / "models/200010.m2").write_bytes(
            chunk("MD21", raw) + chunk("SFID", struct.pack("<I", 505852))
        )
        self.db = sqlite3.connect(self.data / "world.db")
        self.addCleanup(self.db.close)
        self.db.executescript("""
            CREATE TABLE content_gameobject(guid,id,map,position_x,position_y);
            CREATE TABLE content_gameobject_template(entry,type,displayId);
            INSERT INTO content_gameobject_template VALUES
                (202588,5,2690),(202589,15,2700),(202590,5,2701),(202591,5,2702);
            INSERT INTO content_gameobject VALUES
                (1,202588,1,100,200),(2,202589,1,200,300),(3,202591,2,100,200);
        """)
        self.db.commit()

    def seed(self, selector, tiles):
        graph = Closure(self.data, {}, "wow", "fixture")
        selected = closure_seeds.seed_game_objects(
            graph, closure_seeds.Catalogs(graph), self.db, selector, tiles
        )
        return selected, graph.run()

    def test_tiles_reproduce_202588_display2690_model200010_skin505852(self):
        selected, result = self.seed("tiles", {(1, 31, 31)})
        self.assertEqual([row["entry"] for row in selected], [202588, 202589])
        assets = {(row["fdid"], row["type"]): row for row in result["assets"]}
        self.assertEqual(
            set(assets), {(200010, "m2"), (200011, "wmo"), (505852, "skin")}
        )
        self.assertTrue(assets[200010, "m2"]["present"])
        self.assertFalse(assets[505852, "skin"]["present"])

    def test_all_includes_unspawned_and_other_map_templates(self):
        selected, result = self.seed("all", set())
        self.assertEqual(
            [row["entry"] for row in selected], [202588, 202589, 202590, 202591]
        )
        self.assertEqual(
            {row["fdid"] for row in result["assets"]},
            {200010, 200011, 200012, 200013, 505852},
        )

    def test_missing_display_is_named_without_blocking_other_templates(self):
        self.db.execute("INSERT INTO content_gameobject_template VALUES(202592,5,8100)")
        self.db.commit()
        _, result = self.seed("all", set())
        self.assertIn(200010, {row["fdid"] for row in result["assets"]})
        self.assertTrue(
            any(
                row["code"] == "missing_metadata_row" and "8100" in row["reason"]
                for row in result["unresolved"]
            )
        )

    def test_full_seed_fingerprint_includes_game_object_template_changes(self):
        self.db.executescript("""
            CREATE TABLE content_creature(guid,id1,id2,id3,modelid,position_x,position_y,map);
            CREATE TABLE content_creature_template_model(CreatureID,CreatureDisplayID,Idx);
            CREATE TABLE content_item(ID,DisplayInfoID);
            CREATE TABLE spell_name(ID);
        """)
        self.db.commit()
        config = {
            "maps": [],
            "characters": [],
            "npc_displays": "all",
            "game_objects": "all",
            "items": [],
            "spells": [],
            "customization_cache": "cache/customization-v5.sqlite",
        }
        graph = Closure(self.data, {}, "wow", "fixture")
        closure_seeds.seed_catalogs(graph, self.data / "world.db", config)
        self.assertIn(
            (200010, "m2"),
            {(row["fdid"], row["type"]) for row in graph.run()["assets"]},
        )
        before = graph.seeds["world_selection_sha256"]
        self.db.execute(
            "UPDATE content_gameobject_template SET displayId=2701 WHERE entry=202588"
        )
        self.db.commit()
        changed = Closure(self.data, {}, "wow", "fixture")
        closure_seeds.seed_catalogs(changed, self.data / "world.db", config)
        self.assertNotEqual(changed.seeds["world_selection_sha256"], before)

    def test_unknown_selector_fails_instead_of_changing_scope(self):
        with self.assertRaisesRegex(ValueError, "game_objects"):
            self.seed("guess", set())


if __name__ == "__main__":
    unittest.main()
