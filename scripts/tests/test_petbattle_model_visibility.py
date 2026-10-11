"""Private native detached-model regression through pet_battle_live.gd.

PETBATTLE_MODELS_DIR selects an owned running driver. Default case requires
Soul of the Aspects versus Rabbit with Soul's receipt deliberately absent.
PETBATTLE_EXPECT_ALLY selects the authentic complete-pair acceptance case.
"""
import json
import os
from pathlib import Path
import time
import unittest
import uuid


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
                    # Mesh existence/full-frustum containment alone accepts tiny,
                    # centrally collapsed pets. Check both battlefield pads and size.
                    x, y, width, height = model["posed_projected_px"]
                    center_x, center_y = x + width / 2, y + height / 2
                    pad_x = (230, 520) if model["name"] == "AllyPet" else (760, 1050)
                    self.assertGreaterEqual(center_x, pad_x[0], model)
                    self.assertLessEqual(center_x, pad_x[1], model)
                    self.assertGreaterEqual(center_y, 260, model)
                    self.assertLessEqual(center_y, 440, model)
                    self.assertGreaterEqual(height, 80, model)
                    self.assertLessEqual(height, 200, model)
                    self.assertGreaterEqual(y + height, 370, model)
                    self.assertLessEqual(y + height, 450, model)
            skin = os.environ.get("PETBATTLE_EXPECT_SKIN")
            if skin:
                self.assertEqual(response["account"]["ui_skin"], skin)
            time.sleep(.5)


if __name__ == "__main__":
    unittest.main()
