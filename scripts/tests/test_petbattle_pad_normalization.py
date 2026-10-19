"""PET_BATTLE_PAD_USES_JOURNAL_CARD_NORMALIZATION against one same-frame native capture.

PETBATTLE_FRAMING_DIR holds the `framing_diagnostic` output of pet_battle_live.gd
(framing.json + framing-viewport.png) for Anomalus 95171 versus Rabbit 328.
Expected scales are recomputed from installed DB2 rows; drawn clipping is read from
the captured viewport pixels, not from bounds.
"""
import csv
import json
import math
import os
from pathlib import Path
import subprocess
import unittest

ANOMALUS = 95171
RABBIT = 328
CARD_SCENE = "33"
CARD_ACTOR = "unwrapped"
# SharedXML ModelSceneActorMixin MODEL_SCENE_ACTOR_DIMENSIONS_FOR_NORMALIZATION.
WIDTH, DEPTH, HEIGHT = 0.9984, 1.121, 2.118
PAD_OFFSET = 2.2


def read_rows(path, key):
    with path.open() as source:
        return {row[key]: row for row in csv.DictReader(source)}


def journal_card_scale(db2, display_id):
    """Pad scale from the Pet Journal card actor rule, from DB2 rows only."""
    display = read_rows(db2 / "CreatureDisplayInfo.csv", "ID")[str(display_id)]
    model = read_rows(db2 / "CreatureModelData.csv", "ID")[display["ModelID"]]
    world = float(display["CreatureModelScale"]) * float(model["ModelScale"])
    box = [float(model[f"GeoBox_{index}"]) for index in range(6)]
    width, depth, height = (world * (box[index + 3] - box[index]) for index in range(3))
    actors = [row for row in read_rows(db2 / "UiModelSceneActor.csv", "ID").values()
              if row["UiModelSceneID"] == CARD_SCENE and row["ScriptTag"] == CARD_ACTOR]
    normalized = float(actors[0]["NormalizedScale"])
    max_scale = max(width / WIDTH, depth / DEPTH, height / HEIGHT)
    requested = float(display["PetInstanceScale"])
    return requested / (1 + (max_scale - 1) * normalized)


def pad_scale(model):
    basis = model["transform"]["basis"]
    return [math.sqrt(sum(value * value for value in axis)) for axis in basis]


def border_alpha_pixels(image, width, height):
    """Alpha-positive drawn pixels on each one-pixel viewport border."""
    rgba = subprocess.run(
        ["ffmpeg", "-v", "error", "-i", str(image), "-f", "rawvideo", "-pix_fmt", "rgba", "-"],
        check=True, capture_output=True).stdout
    assert len(rgba) == width * height * 4, (len(rgba), width, height)

    def alpha(x, y):
        return rgba[(y * width + x) * 4 + 3]

    return {
        "left": sum(alpha(0, y) > 0 for y in range(height)),
        "right": sum(alpha(width - 1, y) > 0 for y in range(height)),
        "top": sum(alpha(x, 0) > 0 for x in range(width)),
        "bottom": sum(alpha(x, height - 1) > 0 for x in range(width)),
    }


@unittest.skipUnless(os.environ.get("PETBATTLE_FRAMING_DIR"), "native framing capture required")
class PetBattlePadNormalization(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.directory = Path(os.environ["PETBATTLE_FRAMING_DIR"])
        data = Path(os.environ.get("PETBATTLE_DATA_DIR", Path(__file__).resolve().parents[2] / "data"))
        cls.db2 = data / "db2/active"
        cls.report = json.loads((cls.directory / "framing.json").read_text())
        cls.models = {model["display_id"]: model for model in cls.report["models"]}

    def test_capture_is_anomalus_versus_rabbit_same_frame(self):
        self.assertEqual(self.report["process_frame"], self.report["draw_frame"])
        self.assertEqual(self.models[ANOMALUS]["name"], "AllyPet")
        self.assertEqual(self.models[RABBIT]["name"], "EnemyPet")

    def test_anomalus_drawn_pixels_reach_no_viewport_edge(self):
        _, _, width, height = (int(value) for value in self.report["viewport"])
        borders = border_alpha_pixels(self.directory / "framing-viewport.png", width, height)
        self.assertEqual(borders, {"left": 0, "right": 0, "top": 0, "bottom": 0})

    def test_anomalus_pad_uses_journal_card_normalization(self):
        expected = journal_card_scale(self.db2, ANOMALUS)
        self.assertLess(expected, 0.3)
        for axis in pad_scale(self.models[ANOMALUS]):
            self.assertAlmostEqual(axis, expected, places=4)

    def test_small_rabbit_pad_grows_to_journal_card_normalization(self):
        expected = journal_card_scale(self.db2, RABBIT)
        self.assertGreater(expected, 1.05)
        for axis in pad_scale(self.models[RABBIT]):
            self.assertAlmostEqual(axis, expected, places=4)

    def test_pads_and_battlefield_camera_are_unchanged(self):
        for display_id, side in ((ANOMALUS, 1), (RABBIT, -1)):
            self.assertEqual(self.models[display_id]["transform"]["origin"][0], 0)
            self.assertAlmostEqual(self.models[display_id]["transform"]["origin"][2], side * PAD_OFFSET, places=5)
        camera = self.report["camera"]
        self.assertEqual(camera["size"], 3.0)
        for actual, expected in zip(camera["transform"]["origin"], (10.0, 0.72, 0.0)):
            self.assertAlmostEqual(actual, expected, places=5)


if __name__ == "__main__":
    unittest.main()
