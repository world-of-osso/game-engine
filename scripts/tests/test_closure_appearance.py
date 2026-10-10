"""Extended display closure uses authored bakes/choices/item resources, never an alternate bake."""

from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).parents[1]))
from asset_closure import Closure
from closure_seeds import Catalogs, seed_displays


class AppearanceTests(unittest.TestCase):
    def test_hd_bake_choices_and_npc_item_materials(self):
        tables = {
            "CreatureDisplayInfo": "ID,ModelID,ExtendedDisplayInfoID\n1,10,20\n2,11,21\n",
            "CreatureModelData": "ID,FileDataID\n10,100\n11,101\n12,104\n",
            "CreatureDisplayInfoExtra": "ID,DisplayRaceID,DisplaySexID,BakeMaterialResourcesID,HDBakeMaterialResourcesID\n20,1,0,200,201\n21,1,0,202,0\n",
            "CreatureDisplayInfoOption": "ID,CreatureDisplayInfoExtraID,ChrCustomizationChoiceID\n1,20,30\n",
            "ChrCustomizationElement": "ID,ChrCustomizationChoiceID,RelatedChrCustomizationChoiceID,ChrCustomizationMaterialID,ChrCustomizationSkinnedModelID,ChrCustomizationCondModelID\n1,30,0,40,50,51\n",
            "ChrCustomizationCondModel": "ID,CreatureModelDataID\n51,12\n",
            "ChrCustomizationMaterial": "ID,MaterialResourcesID\n40,203\n",
            "ChrCustomizationSkinnedModel": "ID,CollectionsFileDataID\n50,102\n",
            "TextureFileData": "ID,MaterialResourcesID,FileDataID\n1,200,500\n2,201,501\n3,202,502\n4,203,503\n5,204,504\n6,205,505\n",
            "NPCModelItemSlotDisplayInfo": "ID,NpcModelID,ItemDisplayInfoID\n1,20,60\n",
            "ItemDisplayInfo": "ID,ModelResourcesID_0,ModelMaterialResourcesID_0\n60,300,204\n",
            "ModelFileData": "ID,ModelResourcesID,FileDataID\n1,300,103\n",
            "ItemDisplayInfoMaterialRes": "ID,ItemDisplayInfoID,MaterialResourcesID\n",
            "ItemDisplayInfoModelMatRes": "ID,ItemDisplayInfoID,MaterialResourcesID\n1,60,205\n",
        }
        with tempfile.TemporaryDirectory() as tmp:
            data = Path(tmp)
            root = data / "db2/fixture"
            root.mkdir(parents=True)
            for n, t in tables.items():
                (root / (n + ".csv")).write_text(t)
            g = Closure(
                data,
                {
                    100: "character/human/male/humanmale_hd.m2",
                    101: "character/human/male/other_hd.m2",
                },
                "wow",
                "fixture",
            )
            seed_displays(g, Catalogs(g), [1, 2])
            self.assertEqual(
                sorted(g.assets),
                [
                    (100, "m2"),
                    (101, "m2"),
                    (102, "m2"),
                    (103, "m2"),
                    (104, "m2"),
                    (501, "blp"),
                    (503, "blp"),
                    (504, "blp"),
                    (505, "blp"),
                ],
            )
            self.assertFalse(
                any(c == "display_extended_appearance" for c, _, _ in g.unresolved)
            )

    def test_absent_extra_is_a_required_metadata_boundary(self):
        with tempfile.TemporaryDirectory() as tmp:
            d = Path(tmp)
            root = d / "db2/fixture"
            root.mkdir(parents=True)
            (root / "CreatureDisplayInfo.csv").write_text(
                "ID,ModelID,ExtendedDisplayInfoID\n1,10,999\n"
            )
            (root / "CreatureModelData.csv").write_text("ID,FileDataID\n10,100\n")
            (root / "CreatureDisplayInfoExtra.csv").write_text("ID\n")
            g = Closure(d, {100: "character/human_hd.m2"}, "wow", "fixture")
            seed_displays(g, Catalogs(g), [1])
            self.assertTrue(
                any(
                    c == "missing_metadata_row"
                    and r == "CreatureDisplayInfoExtra ID=999"
                    for c, r, _ in g.unresolved
                )
            )


if __name__ == "__main__":
    unittest.main()
