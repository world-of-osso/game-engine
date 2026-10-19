"""PET_BATTLE_PAD_USES_JOURNAL_CARD_NORMALIZATION against one same-frame native capture.

PETBATTLE_FRAMING_DIR holds the `framing_diagnostic` output of pet_battle_live.gd
(framing.json + framing-viewport.png) for Anomalus 95171 versus Rabbit 328.
Expected pad scales use the Pet Journal's own size input (the fixture's oracle of
preview.rs pet_actor_bounds) and the installed scene 33 card actor row; drawn
clipping is read from the captured viewport pixels, not from bounds.
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
# SharedXML ModelSceneActorMixin MODEL_SCENE_ACTOR_DIMENSIONS_FOR_NORMALIZATION,
# in engine axes: x width, z depth, y height.
WIDTH, DEPTH, HEIGHT = 0.9984, 1.121, 2.118
PAD_OFFSET = 2.2


def read_card_actor(db2):
    with (db2 / "UiModelSceneActor.csv").open() as source:
        actor = next(row for row in csv.DictReader(source)
                     if row["UiModelSceneID"] == CARD_SCENE and row["ScriptTag"] == CARD_ACTOR)
    with (db2 / "UiModelSceneActorDisplay.csv").open() as source:
        display = next(row for row in csv.DictReader(source)
                       if row["ID"] == actor["UiModelSceneActorDisplayID"])
    return float(actor["NormalizedScale"]), float(display["Scale"])


def normalized(requested, aggressiveness, size):
    """ModelSceneActorMixin:CalculateNormalizedScale over a journal bounds size."""
    width, height, depth = size
    max_scale = max(width / WIDTH, depth / DEPTH, height / HEIGHT)
    return requested / (1 + (max_scale - 1) * aggressiveness)


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
        cls.aggressiveness, cls.card_scale = read_card_actor(data / "db2/active")
        cls.report = json.loads((cls.directory / "framing.json").read_text())
        cls.models = {model["display_id"]: model for model in cls.report["models"]}

    def expected_pad_scale(self, display_id):
        model = self.models[display_id]
        size = model["journal_card_bounds"]["size"]
        return normalized(model["pet_instance_scale"], self.aggressiveness, size)

    def assert_pad_scale(self, display_id, expected):
        for axis in pad_scale(self.models[display_id]):
            self.assertAlmostEqual(axis, expected, places=4)

    def test_capture_is_anomalus_versus_rabbit_same_frame(self):
        self.assertEqual(self.report["process_frame"], self.report["draw_frame"])
        self.assertEqual(self.models[ANOMALUS]["name"], "AllyPet")
        self.assertEqual(self.models[RABBIT]["name"], "EnemyPet")
        self.assertEqual((self.aggressiveness, self.card_scale), (0.5, 7.0))

    def test_anomalus_drawn_pixels_reach_no_viewport_edge(self):
        _, _, width, height = (int(value) for value in self.report["viewport"])
        borders = border_alpha_pixels(self.directory / "framing-viewport.png", width, height)
        self.assertEqual(borders, {"left": 0, "right": 0, "top": 0, "bottom": 0})

    def test_anomalus_pad_uses_journal_card_normalization(self):
        expected = self.expected_pad_scale(ANOMALUS)
        self.assertLess(expected, 1.0)
        self.assert_pad_scale(ANOMALUS, expected)

    def test_small_rabbit_pad_grows_to_journal_card_normalization(self):
        expected = self.expected_pad_scale(RABBIT)
        self.assertGreater(expected, 1.0)
        self.assert_pad_scale(RABBIT, expected)

    def test_pad_normalization_equals_journal_card_normalization(self):
        # Same size input and aggressiveness: the pad and the scene 33 card divide their
        # requested scales (PetInstanceScale versus card Scale 7) by the same factor.
        for display_id in (ANOMALUS, RABBIT):
            model = self.models[display_id]
            size = model["journal_card_bounds"]["size"]
            card = normalized(self.card_scale, self.aggressiveness, size) / self.card_scale
            pad = pad_scale(model)[0] / model["pet_instance_scale"]
            self.assertAlmostEqual(pad, card, places=4)

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
