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
        response = snapshot(directory)
        ally = os.environ.get("PETBATTLE_EXPECT_ALLY")
        labels = {row["name"]: row["text"] for row in response["controls"]}
        self.assertEqual(labels["PetBattleAllyName"], ally or "Soul of the Aspects")
        self.assertEqual(labels["PetBattleEnemyName"], "Rabbit")
        expected = {"AllyPet", "EnemyPet"} if ally else {"EnemyPet"}
        self.assertEqual({row["name"] for row in response["battle_models"]}, expected)
        for model in response["battle_models"]:
            self.assertGreater(model["visible_meshes"], 0)
            self.assertTrue(all(extent > 0 for extent in model["bounds"]), model)


if __name__ == "__main__":
    unittest.main()
