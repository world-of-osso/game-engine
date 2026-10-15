"""Source-derived one-character-per-specialization test roster."""
from bot_rotation import read_rows


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
        race = min(int(row["RaceID"]) for row in pairs
                   if int(row["ClassID"]) == class_id and int(row["RaceID"]) in alliance)
        name = "B" + classes[class_id]["Filename"][:3].lower()
        name += "".join(letter for letter in spec["Name_lang"] if letter.isalpha())[:6].lower()
        result.append({
            "spec": int(spec["ID"]), "class": class_id, "race": race, "level": level,
            "class_name": classes[class_id]["Name_lang"], "spec_name": spec["Name_lang"],
            "role": int(spec["Role"]), "name": name,
            "account": f"fb_bot_harness_{spec['ID']}",
        })
    return result
