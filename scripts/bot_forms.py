"""Authored pre-combat forms backed by installed spec descriptions and form auras."""
import json

from bot_rotation import read_rows

# ChrSpecialization 103 describes a great cat; 104 describes a mighty bear.
# Their native form spells apply SPELL_AURA_MOD_SHAPESHIFT (36), form IDs 1/5.
# This is fixture preparation, not an added or reordered combat priority.
REQUIRED_FORMS = {103: (768, 1), 104: (5487, 5)}


def load_plan(directory, spec):
    if spec not in REQUIRED_FORMS:
        return {"spec": spec, "required": False}
    spell, form = REQUIRED_FORMS[spec]
    specialization = next((row for row in read_rows(directory, "ChrSpecialization")
                           if int(row["ID"]) == spec), None)
    if specialization is None or specialization["ClassID"] != "11":
        raise ValueError(f"Missing Druid specialization source {spec} for form spell {spell}")
    effects = [row for row in read_rows(directory, "SpellEffect")
               if int(row["SpellID"]) == spell and row["Effect"] == "6" and row["EffectAura"] == "36"]
    if len(effects) != 1 or int(effects[0]["EffectMiscValue_0"]) != form:
        raise ValueError(f"Missing or conflicting source form aura36 for spell {spell}/form {form}")
    form_row = next((row for row in read_rows(directory, "SpellShapeshiftForm")
                     if int(row["ID"]) == form), None)
    if form_row is None:
        raise ValueError(f"Missing source form {form} for spell {spell}")
    return {"spec": spec, "required": True, "spell": spell, "form": form,
            "source": {"ChrSpecialization": specialization, "SpellEffect": effects[0],
                       "SpellShapeshiftForm": form_row}}


def form_is_observed(state, plan):
    encoded = state["assisted"].get("evaluation_json")
    if not encoded:
        raise RuntimeError("Native form aura facts unavailable")
    evaluation = json.loads(encoded)
    if evaluation["specialization"] != plan["spec"]:
        raise RuntimeError("Native form evaluation specialization differs from the fixture")
    return plan["spell"] in evaluation["facts"]["player_auras"]
