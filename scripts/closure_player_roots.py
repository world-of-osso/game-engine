"""Player body/customization and starting-item selectors mirror native source rules.

Body sources: core/player_model_data.rs and customization_catalog::overlay_forever.
Starting-item sources: server-combat::NewCharacterKit::new_item.
"""

from closure_root_products import ITEM_SOURCES, SOURCES, source_catalog
from closure_seeds import number, seed_customizations

SKYBORNE_RACE_MASK_BITS = {95: 32, 96: 33}
SKYBORNE_RACES = set(SKYBORNE_RACE_MASK_BITS)


def player_product(race):
    return ITEM_SOURCES[player_item_definition_source(race)]


def player_item_definition_source(race):
    return "Forever70205" if race in SKYBORNE_RACES else "Retail"


def player_race_mask_bit(graph, catalogs, race):
    if race in SKYBORNE_RACE_MASK_BITS:
        return SKYBORNE_RACE_MASK_BITS[race]
    row = catalogs.index("ChrRaces").get(race)
    if row is None:
        graph.issue(
            "missing_metadata_row", f"ChrRaces ID={race} for starting-item race mask"
        )
        return None
    try:
        bit = int(row["PlayableRaceBit"])
    except (KeyError, TypeError, ValueError):
        graph.issue(
            "missing_metadata_row",
            f"ChrRaces ID={race} PlayableRaceBit for starting-item race mask",
        )
        return None
    if not 0 <= bit < 64:
        graph.issue(
            "invalid_race_mask_bit",
            f"ChrRaces ID={race} PlayableRaceBit={bit} is outside RaceMasks",
        )
        return None
    return bit


def player_requests(graph, selector):
    if selector != "all":
        return [dict(character) for character in selector]
    graph.issue(
        "playable_character_reachability",
        "all modeled Retail race/sex pairs plus four required Skyborne overlays; supported-pair policy remains unresolved",
    )
    with source_catalog(graph, "wow") as catalogs:
        pairs = {
            (number(row, "ChrRacesID"), number(row, "Sex"))
            for row in catalogs.rows("ChrRaceXChrModel")
        }
    pairs.update((race, sex) for race in SKYBORNE_RACES for sex in [0, 1])
    return [{"race": race, "sex": sex, "choices": "all"} for race, sex in sorted(pairs)]


def body_reference(catalogs, character):
    links = {
        (number(row, "ChrRacesID"), number(row, "Sex")): row
        for row in catalogs.rows("ChrRaceXChrModel")
    }
    link = links.get((character["race"], character["sex"]))
    model_id = number(link or {}, "ChrModelID")
    model = catalogs.index("ChrModel").get(model_id)
    display = catalogs.index("CreatureDisplayInfo").get(
        number(model or {}, "DisplayID")
    )
    body = catalogs.index("CreatureModelData").get(number(display or {}, "ModelID"))
    fdid = number(body or {}, "FileDataID")
    if not link or not model or not display or not fdid:
        catalogs.graph.issue(
            "missing_player_source_row",
            f"{catalogs.graph.product} player race {character['race']} sex {character['sex']}: required source-local body chain is incomplete",
        )
        return None
    return model_id, fdid, number(model, "SkeletonFileDataID")


def seed_character(graph, catalogs, character):
    reference = body_reference(catalogs, character)
    if reference is None:
        return None
    model_id, fdid, skeleton = reference
    reason = f"player race {character['race']} sex {character['sex']} source {graph.product} ChrModel {model_id}"
    graph.add(fdid, "m2", reason)
    graph.add(skeleton, "skel", reason + " declared skeleton")
    # Runtime loads requirements even when the conservative audit enumerates all choices.
    catalogs.rows("ChrCustomizationReq")
    if graph.product == "wow_classic_beta":
        graph.seeds["customization_cache"] = (
            f"{SOURCES[graph.product]}/cache/customization-v5.sqlite"
        )
    choices = seed_customizations(
        graph,
        model_id,
        character["choices"],
        character["race"],
        character.get("class", 1),
    )
    character["selected_choices"] = choices
    return {
        "race": character["race"],
        "sex": character["sex"],
        "product": graph.product,
        "chr_model_id": model_id,
        "model_fdid": fdid,
        "customization_cache": graph.seeds["customization_cache"],
        "selected_choices": choices,
    }


def seed_characters(graph, selector):
    characters = player_requests(graph, selector)
    sources = []
    for product in SOURCES:
        selected = [row for row in characters if player_product(row["race"]) == product]
        if not selected:
            continue
        with source_catalog(graph, product) as catalogs:
            for character in selected:
                source = seed_character(graph, catalogs, character)
                if source is not None:
                    sources.append(source)
    graph.seeds["character_sources"] = sources
    return characters, set()
