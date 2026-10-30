use game_engine_core::{adt, campsite_object_data::campsite_doodad_placement};

#[test]
fn stormwind_doodads_keep_authored_mddf_heights() {
    let path = format!(
        "{}/../../data/terrain/777628.adt",
        env!("CARGO_MANIFEST_DIR")
    );
    let objects = adt::parse_obj(&std::fs::read(path).unwrap()).unwrap();
    for (id, height) in [
        (15_534_074, 69.585_7),
        (15_534_075, 69.216_27),
        (5_479_346, 123.661_97),
        (15_531_736, 16.791_15),
        (2_630_160, 18.871_515),
        (2_535_082, 19.002_392),
    ] {
        let doodad = objects.doodads.iter().find(|d| d.unique_id == id).unwrap();
        assert_eq!(doodad.position[1], height, "MDDF height {id}");
        assert_eq!(
            campsite_doodad_placement(doodad, 30, 48).translation.y,
            height,
            "placed height {id}"
        );
    }
}
