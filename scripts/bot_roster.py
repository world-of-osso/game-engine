"""Source-derived one-character-per-specialization test roster."""
from bot_rotation import read_rows


def select_alliance_race(class_id, pairs, alliance):
    available = {int(row["RaceID"]) for row in pairs
                 if int(row["ClassID"]) == class_id and int(row["RaceID"]) in alliance}
    race = 11 if class_id == 7 else min(available)
    if race not in available:
        raise ValueError(f"Authored Alliance race {race} for class {class_id} is not in source CharBaseInfo")
    return race


def build_roster(directory, level):
    classes = {int(row["ID"]): row for row in read_rows(directory, "ChrClasses")}
    pairs = list(read_rows(directory, "CharBaseInfo"))
    alliance = {int(row["ID"]) for row in read_rows(directory, "ChrRaces")
                if row["Alliance"] == "0" and int(row["PlayableRaceBit"]) >= 0}
    specs = [row for row in read_rows(directory, "ChrSpecialization")
             if int(row["ClassID"]) > 0 and int(row["OrderIndex"]) < 4]
    result = []
    for spec in specs:
        class_id = int(spec["ClassID"])
        race = select_alliance_race(class_id, pairs, alliance)
        name = "B" + classes[class_id]["Filename"][:3].lower()
        name += "".join(letter for letter in spec["Name_lang"] if letter.isalpha())[:6].lower()
        result.append({
            "spec": int(spec["ID"]), "class": class_id, "race": race, "level": level,
            "class_name": classes[class_id]["Name_lang"], "spec_name": spec["Name_lang"],
            "role": int(spec["Role"]), "name": name,
            "account": f"fb_bot_harness_{spec['ID']}",
        })
    return result
