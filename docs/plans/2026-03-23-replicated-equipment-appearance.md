# Replicated Equipment Appearance Implementation Plan

**Goal:** Replicate each player's visible equipped gear from server to client so remote and local characters render the same worn appearance across armor, tabard/shirt, and handheld equipment.

**Architecture:** Make the server authoritative for a compact per-slot appearance snapshot and replicate that snapshot alongside the existing `Player` component. The client resolves each slot into body texture overlays, geoset toggles, and attached 3D item models using local DB2/CSV data, with texture/geoset rendering shipped first and attachment-heavy 3D slots layered on after the protocol is stable.

**Tech Stack:** Rust, Bevy 0.18, Lightyear replication, shared protocol/components crate, WoW DB2 CSV data in `data/`, existing `outfit_data`, `character_customization`, and `equipment` systems.

---

Slot taxonomy:

- Total worn equipment slots: `Head`, `Neck`, `Shoulder`, `Shirt`, `Chest`, `Waist`, `Legs`, `Feet`, `Wrist`, `Hands`, `Finger1`, `Finger2`, `Trinket1`, `Trinket2`, `Back`, `Tabard`, `MainHand`, `OffHand`
- Visible appearance slots to replicate for rendering: `Head`, `Shoulder`, `Back`, `Chest`, `Shirt`, `Tabard`, `Wrist`, `Hands`, `Waist`, `Legs`, `Feet`, `MainHand`, `OffHand`
- Non-visible worn slots to keep in inventory/gameplay state but exclude from renderer appearance replication: `Neck`, `Finger1`, `Finger2`, `Trinket1`, `Trinket2`
- Do not model `Robe` as a separate replicated slot; it should be represented by the equipped chest appearance plus inventory-type metadata

### Task 1: Define the replicated appearance model

**Files:**
- Modify: `../game-server/crates/shared/src/components.rs`
- Modify: `../game-server/crates/shared/src/protocol.rs`
- Modify: `../game-server/crates/shared/src/protocol_snapshots.rs`
- Test: `../game-server/crates/shared/src/protocol.rs`

**Step 1: Write the failing test**

Add a shared-crate test that round-trips the new replicated appearance component and verifies every slot enum variant survives serialization.

```rust
#[test]
fn equipment_appearance_roundtrips() {
    let snapshot = EquipmentAppearance {
        entries: vec![
            EquippedAppearanceEntry {
                slot: EquipmentVisualSlot::Head,
                item_id: Some(19019),
                display_info_id: Some(12345),
                inventory_type: 1,
                hidden: false,
            },
            EquippedAppearanceEntry {
                slot: EquipmentVisualSlot::MainHand,
                item_id: Some(17182),
                display_info_id: Some(54321),
                inventory_type: 21,
                hidden: false,
            },
        ],
    };
    let bytes = bitcode::encode(&snapshot);
    let decoded: EquipmentAppearance = bitcode::decode(&bytes).unwrap();
    assert_eq!(decoded, snapshot);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p shared equipment_appearance_roundtrips -- --exact`
Expected: FAIL because `EquipmentAppearance`, `EquippedAppearanceEntry`, and `EquipmentVisualSlot` do not exist.

**Step 3: Write minimal implementation**

Add a new replicated component in `components.rs`.

```rust
#[derive(Component, Reflect, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct EquipmentAppearance {
    pub entries: Vec<EquippedAppearanceEntry>,
}

#[derive(Reflect, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct EquippedAppearanceEntry {
    pub slot: EquipmentVisualSlot,
    pub item_id: Option<u32>,
    pub display_info_id: Option<u32>,
    pub inventory_type: u8,
    pub hidden: bool,
}

#[derive(Reflect, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EquipmentVisualSlot {
    Head,
    Shoulder,
    Back,
    Chest,
    Shirt,
    Tabard,
    Wrist,
    Hands,
    Waist,
    Legs,
    Feet,
    MainHand,
    OffHand,
}
```

Register the component for replication in `protocol.rs`.

```rust
app.register_component::<EquipmentAppearance>();
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p shared equipment_appearance_roundtrips -- --exact`
Expected: PASS

**Step 5: Commit**

```bash
git add ../game-server/crates/shared/src/components.rs ../game-server/crates/shared/src/protocol.rs ../game-server/crates/shared/src/protocol_snapshots.rs
git commit -m "feat: define replicated equipment appearance component"
```

### Task 2: Persist server-side equipped appearance state per character

**Files:**
- Modify: `../game-server/crates/server/src/persistence_types.rs`
- Modify: `../game-server/crates/server/src/persistence.rs`
- Modify: `../game-server/crates/server/src/auth.rs`
- Test: `../game-server/crates/server/src/persistence_tests.rs`

**Step 1: Write the failing test**

Add a persistence test that saves a character with equipped slot appearances and loads it back unchanged.

```rust
#[test]
fn character_equipment_appearance_persists() {
    let data = CharacterData {
        name: "Theron".into(),
        equipment_appearance: EquipmentAppearance {
            entries: vec![
                EquippedAppearanceEntry {
                    slot: EquipmentVisualSlot::Chest,
                    item_id: Some(6123),
                    display_info_id: Some(777),
                    inventory_type: 5,
                    hidden: false,
                },
            ],
            ..Default::default()
        },
        ..sample_character()
    };
    roundtrip_character_data(data);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p server character_equipment_appearance_persists -- --exact`
Expected: FAIL because character persistence does not store any equipment appearance state.

**Step 3: Write minimal implementation**

Extend `CharacterData` with authoritative appearance state.

```rust
pub struct CharacterData {
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub health: f32,
    pub max_health: f32,
    pub race: u8,
    pub class: u8,
    pub level: u16,
    pub appearance: CharacterAppearance,
    pub equipment_appearance: EquipmentAppearance,
}
```

Update decode compatibility to default old characters to an empty `EquipmentAppearance`.

```rust
equipment_appearance: EquipmentAppearance::default(),
```

Insert the replicated component when spawning the player entity in `auth.rs`.

```rust
EquipmentAppearance(data.equipment_appearance.clone())
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p server character_equipment_appearance_persists -- --exact`
Expected: PASS

**Step 5: Commit**

```bash
git add ../game-server/crates/server/src/persistence_types.rs ../game-server/crates/server/src/persistence.rs ../game-server/crates/server/src/auth.rs ../game-server/crates/server/src/persistence_tests.rs
git commit -m "feat: persist character equipment appearance"
```

### Task 3: Populate the server snapshot from equipped items and selected appearances

**Files:**
- Modify: `../game-server/crates/server/src/persistence.rs`
- Modify: `../game-server/crates/server/src/admin_ipc.rs`
- Modify: `../game-server/crates/server/src/bin/admin.rs`
- Modify: `src/item_info.rs`
- Create: `src/item_appearance.rs`
- Test: `../game-server/crates/server/src/persistence_tests.rs`
- Test: `src/item_info.rs`

**Step 1: Write the failing test**

Add one server test that maps an equipped chest item plus a transmog-selected appearance into a replicated `EquippedAppearanceEntry`, and one engine test that resolves `ItemAppearanceID` or `ItemDisplayInfoID` from local data.

```rust
#[test]
fn equipped_item_maps_to_replicated_chest_appearance() {
    let entry = build_equipped_appearance_entry(sample_equipped_chest_item());
    assert_eq!(entry.slot, EquipmentVisualSlot::Chest);
    assert_eq!(entry.display_info_id, Some(777));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p server equipped_item_maps_to_replicated_chest_appearance -- --exact`
Expected: FAIL because no mapping code exists.

**Step 3: Write minimal implementation**

Introduce a server-side builder that converts authoritative equipped items into replicated slot entries.

```rust
fn build_equipped_appearance_entry(item: &EquippedItemData) -> Option<EquippedAppearanceEntry> {
    Some(EquippedAppearanceEntry {
        slot: map_inventory_type_to_slot(item.inventory_type)?,
        item_id: Some(item.item_id),
        display_info_id: item.selected_display_info_id,
        inventory_type: item.inventory_type,
        hidden: false,
    })
}
```

Use two enums in the server model:

```rust
pub enum WornInventorySlot {
    Head,
    Neck,
    Shoulder,
    Shirt,
    Chest,
    Waist,
    Legs,
    Feet,
    Wrist,
    Hands,
    Finger1,
    Finger2,
    Trinket1,
    Trinket2,
    Back,
    Tabard,
    MainHand,
    OffHand,
}
```

```rust
pub enum EquipmentVisualSlot {
    Head,
    Shoulder,
    Back,
    Chest,
    Shirt,
    Tabard,
    Wrist,
    Hands,
    Waist,
    Legs,
    Feet,
    MainHand,
    OffHand,
}
```

Only map visible worn slots into `EquipmentVisualSlot` for replication. Keep rings, trinkets, and neck out of the replicated renderer payload.

Add client-side lookup helpers in `src/item_appearance.rs` for:

- `ItemID -> ItemAppearanceID`
- `ItemAppearanceID -> ItemDisplayInfoID`
- `ItemDisplayInfoID -> model/textures/geoset hints`

Use local CSV files in `data/` instead of any network fetches.

**Step 4: Run test to verify it passes**

Run: `cargo test -p server equipped_item_maps_to_replicated_chest_appearance -- --exact`
Run: `cargo test -p game-engine item_appearance -- --nocapture`
Expected: PASS

**Step 5: Commit**

```bash
git add ../game-server/crates/server/src/persistence.rs ../game-server/crates/server/src/admin_ipc.rs ../game-server/crates/server/src/bin/admin.rs src/item_info.rs src/item_appearance.rs
git commit -m "feat: build replicated equipment appearance from equipped items"
```

### Task 4: Render replicated texture and geoset appearance on the client

**Files:**
- Modify: `src/networking.rs`
- Modify: `src/character_customization.rs`
- Modify: `src/outfit_data.rs`
- Create: `src/equipment_appearance.rs`
- Test: `src/networking_tests.rs`
- Test: `src/character_customization.rs`

**Step 1: Write the failing test**

Add a client test that spawns a replicated player with a chest, tabard, gloves, and boots appearance snapshot and verifies those slot overlays are forwarded into the body texture compositor.

```rust
#[test]
fn replicated_player_applies_equipped_texture_overlays() {
    let snapshot = sample_equipment_appearance([
        EquipmentVisualSlot::Chest,
        EquipmentVisualSlot::Hands,
        EquipmentVisualSlot::Feet,
        EquipmentVisualSlot::Tabard,
    ]);
    let overlays = resolve_equipment_texture_overlays(&resolver, &snapshot);
    assert!(!overlays.item_textures.is_empty());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p game-engine replicated_player_applies_equipped_texture_overlays -- --exact`
Expected: FAIL because replicated players have no equipment appearance input and `apply_body_texture` ignores outfit item overlays.

**Step 3: Write minimal implementation**

Add a client resolver that converts replicated slot entries into compositing inputs.

```rust
pub struct ResolvedEquipmentAppearance {
    pub item_textures: Vec<(u8, u32)>,
    pub geoset_overrides: Vec<(u16, u16)>,
    pub attachment_models: Vec<(EquipmentVisualSlot, u32)>,
}
```

Update `apply_character_customization` to merge starter outfit and equipped appearance overlays.

```rust
let merged_item_textures = merge_item_textures(&outfit.item_textures, &equipment.item_textures);
let composited = char_tex.composite_model_textures(&all_materials, &merged_item_textures, layout_id)?;
```

Do not apply raw `ItemDisplayInfo::GeosetGroup_*` values directly; add slot-aware translation helpers first so helmets, gloves, boots, belts, and robes map to actual M2 geoset groups safely.

**Step 4: Run test to verify it passes**

Run: `cargo test -p game-engine replicated_player_applies_equipped_texture_overlays -- --exact`
Expected: PASS

**Step 5: Commit**

```bash
git add src/networking.rs src/character_customization.rs src/outfit_data.rs src/equipment_appearance.rs src/networking_tests.rs
git commit -m "feat: apply replicated armor appearance to player textures"
```

### Task 5: Render 3D attachment-driven equipment models

**Files:**
- Modify: `src/equipment.rs`
- Modify: `src/m2_scene.rs`
- Modify: `src/networking.rs`
- Modify: `src/scene_tree.rs`
- Modify: `src/status.rs`
- Test: `src/networking_tests.rs`
- Test: `src/bin/game-engine-cli/tests.rs`

**Step 1: Write the failing test**

Add a test that a replicated player with shoulder and weapon entries produces desired attachment-model state for `Shoulder`, `MainHand`, and `OffHand`.

```rust
#[test]
fn replicated_player_spawns_attachment_models_for_weapons_and_shoulders() {
    let snapshot = sample_equipment_appearance([
        EquipmentVisualSlot::Shoulder,
        EquipmentVisualSlot::MainHand,
        EquipmentVisualSlot::OffHand,
    ]);
    let desired = build_desired_attachment_models(&resolver, &snapshot);
    assert_eq!(desired.len(), 3);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p game-engine replicated_player_spawns_attachment_models_for_weapons_and_shoulders -- --exact`
Expected: FAIL because the `Equipment` component only supports `MainHand` and `OffHand`.

**Step 3: Write minimal implementation**

Expand `src/equipment.rs` to support all attachment-backed slots needed now.

```rust
pub enum EquipmentSlot {
    Head,
    ShoulderLeft,
    ShoulderRight,
    Back,
    MainHand,
    OffHand,
}
```

Map replicated visual slots to one or more attachment-backed runtime slots.

```rust
match visual_slot {
    EquipmentVisualSlot::Shoulder => vec![EquipmentSlot::ShoulderLeft, EquipmentSlot::ShoulderRight],
    EquipmentVisualSlot::MainHand => vec![EquipmentSlot::MainHand],
    EquipmentVisualSlot::OffHand => vec![EquipmentSlot::OffHand],
    EquipmentVisualSlot::Back => vec![EquipmentSlot::Back],
    _ => vec![],
}
```

Update scene/status dumps so IPC and exported snapshots show resolved equipment by semantic slot instead of just raw model path strings.

**Step 4: Run test to verify it passes**

Run: `cargo test -p game-engine replicated_player_spawns_attachment_models_for_weapons_and_shoulders -- --exact`
Expected: PASS

**Step 5: Commit**

```bash
git add src/equipment.rs src/m2_scene.rs src/networking.rs src/scene_tree.rs src/status.rs src/networking_tests.rs src/bin/game-engine-cli/tests.rs
git commit -m "feat: render replicated 3d equipment attachments"
```

### Task 6: End-to-end verification and tooling

**Files:**
- Modify: `src/character_export.rs`
- Modify: `src/ipc/format.rs`
- Modify: `src/bin/game-engine-cli/main.rs`
- Modify: `src/bin/game-engine-cli/tests.rs`
- Test: `src/bin/game-engine-cli/tests.rs`

**Step 1: Write the failing test**

Add a CLI/export test that serializes a character with full replicated equipment appearance and verifies slot names and display ids are present in the exported payload.

```rust
#[test]
fn export_character_payload_includes_equipment_appearance() {
    let payload = build_export_character_payload(&stats, &equipped_gear, &equipment_appearance).unwrap();
    assert!(payload.equipped_appearance.iter().any(|e| e.slot == "Chest"));
    assert!(payload.equipped_appearance.iter().any(|e| e.display_info_id == Some(777)));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p game-engine export_character_payload_includes_equipment_appearance -- --exact`
Expected: FAIL because exports and CLI output only know about ad-hoc rendered model paths.

**Step 3: Write minimal implementation**

Expose replicated appearance in status, CLI, and export payloads.

```rust
pub struct ExportEquippedAppearanceEntry {
    pub slot: String,
    pub item_id: Option<u32>,
    pub display_info_id: Option<u32>,
    pub hidden: bool,
}
```

Add a manual verification pass:

1. Start `../game-server`
2. Log into `game-engine` with a character wearing at least chest, gloves, boots, tabard, shoulders, and one weapon
3. Run `cargo run --bin game-engine-cli -- equipment-status`
4. Run `cargo run --bin game-engine -- --screen inworld --dump-scene`
5. Confirm remote and local players show matching slot data and visible appearance

**Step 4: Run test to verify it passes**

Run: `cargo test -p game-engine export_character_payload_includes_equipment_appearance -- --exact`
Run: `./run-tests.sh`
Expected: PASS

**Step 5: Commit**

```bash
git add src/character_export.rs src/ipc/format.rs src/bin/game-engine-cli/main.rs src/bin/game-engine-cli/tests.rs
git commit -m "feat: expose replicated equipment appearance in tooling"
```

## Notes and sequencing

- Ship texture/geoset slots before attachment-heavy 3D gear if you want faster value; the protocol should support all slots from day one
- Prefer replicating `display_info_id` for render fidelity, with `item_id` retained for debugging and UI
- Keep `hidden` in the wire format now so helm/cloak hide toggles do not require a protocol change later
- Treat missing `display_info_id` as an empty slot on the client rather than falling back to guessed item assets
- Reuse local data in `data/` and never download anything to `/tmp/`
- Maintain a hard separation between worn inventory slots and visible appearance slots so gameplay systems can keep all 18 slots without forcing non-visible data into the render protocol
