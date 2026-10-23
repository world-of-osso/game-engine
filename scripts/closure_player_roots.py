"""Player body/customization ownership mirrors the explicit native Skyborne overlay.

Sources: core/player_model_data.rs and customization_catalog::overlay_forever.
Only races95/96 use Forever; this policy never selects owned-item definitions.
"""

from closure_root_products import SOURCES, source_catalog
from closure_seeds import number, seed_customizations

SKYBORNE_RACES = {95, 96}


def player_product(race):
    return "wow_classic_beta" if race in SKYBORNE_RACES else "wow"


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
