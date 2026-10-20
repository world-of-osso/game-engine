"""Private native detached-model regression through pet_battle_live.gd.

PETBATTLE_MODELS_DIR selects an owned running driver. Default case requires
Soul of the Aspects versus Rabbit with Soul's receipt deliberately absent.
PETBATTLE_EXPECT_ALLY selects the authentic complete-pair acceptance case.
"""
import csv
import json
import os
from pathlib import Path
import time
import unittest
import uuid

from scripts.tests.test_petbattle_pad_normalization import normalized, read_card_actor


def snapshot(directory):
    request = {"id": str(uuid.uuid4()), "action": "snapshot"}
    temporary = directory / "command.tmp"
    temporary.write_text(json.dumps(request))
    temporary.replace(directory / "command.json")
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        try:
            response = json.loads((directory / "response.json").read_text())
        except (FileNotFoundError, json.JSONDecodeError):
            time.sleep(.1)
            continue
        if response.get("id") == request["id"]:
            return response
        time.sleep(.1)
    raise TimeoutError("native fixture did not reply")


@unittest.skipUnless(os.environ.get("PETBATTLE_MODELS_DIR"), "private native fixture required")
class PetBattleModelVisibility(unittest.TestCase):
    def test_active_pair_visibility(self):
        directory = Path(os.environ["PETBATTLE_MODELS_DIR"])
        ally = os.environ.get("PETBATTLE_EXPECT_ALLY")
        data = Path(os.environ.get("PETBATTLE_DATA_DIR", Path(__file__).resolve().parents[2] / "data"))
        with (data / "CreatureDisplayInfo.csv").open() as source:
            displays = {int(row["ID"]): row for row in csv.DictReader(source)}
        with (data / "CreatureModelData.csv").open() as source:
            model_scales = {int(row["ID"]): float(row["ModelScale"]) for row in csv.DictReader(source)}
        aggressiveness, _ = read_card_actor(data / "db2/active")
        poses = {"AllyPet": set(), "EnemyPet": set()}
        deadline = time.monotonic() + 8
        while time.monotonic() < deadline:
            response = snapshot(directory)
            labels = {row["name"]: row["text"] for row in response["controls"]}
            self.assertEqual(labels["PetBattleAllyName"], ally or "Soul of the Aspects")
            self.assertEqual(labels["PetBattleEnemyName"], "Rabbit")
            expected = {"AllyPet", "EnemyPet"} if ally else {"EnemyPet"}
            self.assertEqual({row["name"] for row in response["battle_models"]}, expected)
            for model in response["battle_models"]:
                self.assertGreater(model["visible_meshes"], 0)
                self.assertTrue(model["framed"], f"authored animation bounds clipped: {model}")
                self.assertTrue(all(extent > 0 for extent in model["bounds"]), model)
                if ally:
                    # Fixed pads and authentic data, not equal projected heights.
                    x, y, width, height = model["posed_projected_px"]
                    center_x = x + width / 2
                    # Pad bands authored on a 1280-wide viewport, scaled to the real one.
                    band = (230, 520) if model["name"] == "AllyPet" else (760, 1050)
                    viewport_width = response["viewport_size"][0]
                    pad_x = tuple(edge / 1280 * viewport_width for edge in band)
                    self.assertGreaterEqual(center_x, pad_x[0], model)
                    self.assertLessEqual(center_x, pad_x[1], model)
                    self.assertGreater(height, 0, model)
                    # PET_BATTLE_PAD_USES_JOURNAL_CARD_NORMALIZATION: PetInstanceScale
                    # normalized like the journal card over the journal's own bounds.
                    display = displays[model["display_id"]]
                    instance_scale = float(display["PetInstanceScale"])
                    journal_size = model["journal_card_bounds"]["size"]
                    pad_scale = normalized(instance_scale, aggressiveness, journal_size)
                    world_scale = float(display["CreatureModelScale"]) * model_scales[int(display["ModelID"])]
                    for axis in range(3):
                        self.assertAlmostEqual(model["scale"][axis], pad_scale, places=3, msg=str(model))
                        self.assertAlmostEqual(model["visual_scale"][axis], world_scale, places=3, msg=str(model))
                        self.assertAlmostEqual(model["visual_position"][axis], 0, places=4, msg=str(model))
                    expected_z = 2.2 if model["name"] == "AllyPet" else -2.2
                    self.assertAlmostEqual(model["position"][2], expected_z, places=4, msg=str(model))
                    self.assertAlmostEqual(model["position"][0], 0, places=4, msg=str(model))
                    self.assertAlmostEqual(model["position"][1], 0, places=4, msg=str(model))
                    poses[model["name"]].add(tuple(round(value, 2) for value in model["posed_projected_px"]))
            if ally:
                pair = {model["name"]: model for model in response["battle_models"]}
                actual_ratio = pair["AllyPet"]["visual_scale"][0] * pair["AllyPet"]["scale"][0] / (pair["EnemyPet"]["visual_scale"][0] * pair["EnemyPet"]["scale"][0])
                expected = []
                for name in ("AllyPet", "EnemyPet"):
                    row = displays[pair[name]["display_id"]]
                    world = float(row["CreatureModelScale"]) * model_scales[int(row["ModelID"])]
                    size = pair[name]["journal_card_bounds"]["size"]
                    expected.append(world * normalized(float(row["PetInstanceScale"]), aggressiveness, size))
                self.assertAlmostEqual(actual_ratio, expected[0] / expected[1], places=3)
            skin = os.environ.get("PETBATTLE_EXPECT_SKIN")
            if skin:
                self.assertEqual(response["account"]["ui_skin"], skin)
            time.sleep(.5)
        if ally:
            for name, observed in poses.items():
                self.assertGreater(len(observed), 1, f"{name} never changed its posed geometry")


if __name__ == "__main__":
    unittest.main()
