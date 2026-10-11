"""Real private-server detached completion regression, not a mesh-count replay.

Run with PETBATTLE_MODELS_DIR pointing at the running pet_battle_live.gd driver.
The owned battle must be Soul of the Aspects against Rabbit. The Soul's receipt
is deliberately absent for the failure-isolation case; Rabbit's is authentic.
"""
import json
import os
from pathlib import Path
import time
import unittest
import uuid


@unittest.skipUnless(os.environ.get("PETBATTLE_MODELS_DIR"), "private native fixture required")
class PetBattleModelVisibility(unittest.TestCase):
    def test_ready_rabbit_is_visible_when_ally_asset_fails(self):
        directory = Path(os.environ["PETBATTLE_MODELS_DIR"])
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
                break
            time.sleep(.1)
        else:
            self.fail("native fixture did not reply")
        labels = {row["name"]: row["text"] for row in response["controls"]}
        self.assertEqual(labels["PetBattleAllyName"], "Soul of the Aspects")
        self.assertEqual(labels["PetBattleEnemyName"], "Rabbit")
        self.assertEqual(response["battle_meshes"], 1,
                         "ready Rabbit must render independently of failed Soul receipt")


if __name__ == "__main__":
    unittest.main()
