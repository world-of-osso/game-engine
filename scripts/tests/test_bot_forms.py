import json
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import bot_forms

FIXTURES = Path(__file__).parent / "fixtures/bot_harness"


class BotFormTests(unittest.TestCase):
    def test_feral_and_guardian_use_authentic_spec_form_aura_rows(self):
        for spec, spell, form, name in [(103, 768, 1, "Cat Form"), (104, 5487, 5, "Bear Form")]:
            plan = bot_forms.load_plan(FIXTURES / "forms", spec)
            self.assertTrue(plan["required"])
            self.assertEqual((plan["spell"], plan["form"]), (spell, form))
            self.assertEqual(plan["source"]["SpellShapeshiftForm"]["Name_lang"], name)
            self.assertEqual(plan["source"]["SpellEffect"]["EffectAura"], "36")
            self.assertEqual(plan["source"]["ChrSpecialization"]["ClassID"], "11")
        self.assertFalse(bot_forms.load_plan(FIXTURES / "forms", 71)["required"])

    def test_actual_native_feral_caster_has_no_form_and_positive_observation_requires_own_aura(self):
        state = json.loads((FIXTURES / "native-feral-caster.json").read_text())
        plan = bot_forms.load_plan(FIXTURES / "forms", 103)
        self.assertFalse(bot_forms.form_is_observed(state, plan))
        evaluation = json.loads(state["assisted"]["evaluation_json"])
        evaluation["facts"]["player_auras"].append(768)
        state["assisted"]["evaluation_json"] = json.dumps(evaluation)
        self.assertTrue(bot_forms.form_is_observed(state, plan))
        evaluation["specialization"] = 104
        state["assisted"]["evaluation_json"] = json.dumps(evaluation)
        with self.assertRaisesRegex(RuntimeError, "specialization"):
            bot_forms.form_is_observed(state, plan)

    def test_missing_source_effect_does_not_select_another_form(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / "db2"
            shutil.copytree(FIXTURES / "forms", root)
            path = root / "SpellEffect.csv"
            path.write_text(path.read_text().splitlines()[0] + "\n")
            with self.assertRaisesRegex(ValueError, "768"):
                bot_forms.load_plan(root, 103)


if __name__ == "__main__":
    unittest.main()
