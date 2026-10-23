"""Concrete retained-r5 recovery boundary; inputs are explicit, never extracted."""

import os
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from closure_receipts import ReceiptClosure

HISTORICAL_KEY = "dcfc90fffd79ba00406ae46f5f657592"
MODEL_KEY = "88d5e8ef05ec5a8446cdec93d142d7c6"


@unittest.skipUnless(os.environ.get("P4_RED") == "1", "P4 model recovery RED")
class ConcreteModelRecoveryTests(unittest.TestCase):
    def setUp(self):
        value = os.environ.get("DEPOT_GAME_ENGINE_DATA")
        self.assertTrue(
            value,
            "Set DEPOT_GAME_ENGINE_DATA to the explicit receipt-qualified data view",
        )
        self.data = Path(value)

    def graph(self):
        return ReceiptClosure(self.data, {}, "wow", "12.1.5.70448")

    def test_admitted_132007_model_uses_historical_owned_bytes(self):
        graph = self.graph()
        graph.add(132007, "m2", "retained-r5 authenticated model recovery")
        result = graph.run()
        asset = next(
            row
            for row in result["assets"]
            if row["fdid"] == 132007 and row["type"] == "m2"
        )
        self.assertEqual(asset["identity_status"], "verified_receipt")
        self.assertEqual(asset["source_product"], "wow")
        self.assertEqual(asset["actual_build"], "12.1.0.69933")
        self.assertEqual(asset["asset_build_key"], HISTORICAL_KEY)
        self.assertEqual(
            asset["locations"], [f"products/wow/{HISTORICAL_KEY}/models/132007.m2"]
        )

    def test_122979_companion_keeps_parent_namespace_and_historical_origin(self):
        graph = self.graph()
        graph.add(579120, "m2", "actual Retail owner namespace")
        graph.add_model_texture(579120, 122979, "retained-r5 source-qualified material")
        result = graph.run()
        asset = next(
            row
            for row in result["assets"]
            if row["fdid"] == 122979 and row["type"] == "blp"
        )
        self.assertEqual(asset["identity_status"], "verified_receipt")
        self.assertEqual(asset["actual_build"], "12.1.0.69933")
        self.assertEqual(asset["asset_build_key"], MODEL_KEY)
        self.assertEqual(
            asset["locations"], [f"products/wow/{MODEL_KEY}/textures/122979.blp"]
        )
        self.assertTrue(asset["receipt_selected"])

    def test_residual_122893_is_refused_even_with_flat_model_bytes(self):
        self.assertTrue((self.data / "models/122893.m2").is_file())
        graph = self.graph()
        graph.add(122893, "m2", "retained-r5 source-receipt residual")
        result = graph.run()
        asset = next(
            row
            for row in result["assets"]
            if row["fdid"] == 122893 and row["type"] == "m2"
        )
        self.assertFalse(asset["present"])
        self.assertEqual(asset["identity_status"], "unverified")
        self.assertTrue(
            any(
                row["code"] == "missing_asset_receipt" and row["fdid"] == 122893
                for row in result["unresolved"]
            )
        )


if __name__ == "__main__":
    unittest.main()
