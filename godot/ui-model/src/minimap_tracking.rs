//! Replicated town services and authored resources; learned tracking spells gate gathering filters.
use game_engine_core::minimap_data::MinimapView;
use shared::protocol::NpcFlags;
use ui_toolkit::rsx;
use ui_toolkit::widget_def::Element;

use super::{BlipKind, MinimapBlip, SheetArt, object_icon};
use crate::ui::strata::FrameStrata;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum TrackingFilter {
    Flightmaster,
    Innkeeper,
    Repair,
    ClassTrainer,
    ProfessionTrainer,
    Banker,
    Food,
    Reagents,
    Auctioneer,
    FindHerbs,
    FindMinerals,
}

struct Service {
    filter: TrackingFilter,
    label: &'static str,
    flag: u64,
    art: SheetArt,
}

// Retail ObjectIconsAtlas 647; UiTextureAtlasMember 4689/4685/4687/4679/4681/
// 4675/4683/4684/4686. Order also identifies each local menu action.
const SERVICES: [Service; 9] = [
    Service {
        filter: TrackingFilter::Flightmaster,
        label: "Flight Masters",
        flag: NpcFlags::FLIGHTMASTER,
        art: object_icon(797.0, 829.0, 424.0, 456.0),
    },
    Service {
        filter: TrackingFilter::Innkeeper,
        label: "Innkeepers",
        flag: NpcFlags::INNKEEPER,
        art: object_icon(457.0, 489.0, 866.0, 898.0),
    },
    Service {
        filter: TrackingFilter::Repair,
        label: "Repair",
        flag: NpcFlags::REPAIR,
        art: object_icon(593.0, 625.0, 866.0, 898.0),
    },
    Service {
        filter: TrackingFilter::ClassTrainer,
        label: "Class Trainers",
        flag: NpcFlags::TRAINER_CLASS,
        art: object_icon(423.0, 455.0, 832.0, 864.0),
    },
    Service {
        filter: TrackingFilter::ProfessionTrainer,
        label: "Profession Trainers",
        flag: NpcFlags::TRAINER_PROFESSION,
        art: object_icon(763.0, 795.0, 526.0, 558.0),
    },
    Service {
        filter: TrackingFilter::Banker,
        label: "Bankers",
        flag: NpcFlags::BANKER,
        art: object_icon(423.0, 455.0, 628.0, 660.0),
    },
    Service {
        filter: TrackingFilter::Food,
        label: "Food & Drink",
        flag: NpcFlags::VENDOR_FOOD,
        art: object_icon(457.0, 489.0, 458.0, 490.0),
    },
    Service {
        filter: TrackingFilter::Reagents,
        label: "Reagents",
        flag: NpcFlags::VENDOR_REAGENT,
        art: object_icon(593.0, 625.0, 832.0, 864.0),
    },
    Service {
        filter: TrackingFilter::Auctioneer,
        label: "Auctioneers",
        flag: NpcFlags::AUCTIONEER,
        art: object_icon(423.0, 455.0, 526.0, 558.0),
    },
];

struct ResourceFilter {
    filter: TrackingFilter,
    label: &'static str,
    parent_skill: u32,
    art: SheetArt,
}

// Retail atlas647 members17635/17636: professions_tracking_herb / _ore.
const RESOURCE_FILTERS: [ResourceFilter; 2] = [
    ResourceFilter {
        filter: TrackingFilter::FindHerbs,
        label: "Find Herbs",
        parent_skill: 182,
        art: object_icon(865.0, 897.0, 526.0, 558.0),
    },
    ResourceFilter {
        filter: TrackingFilter::FindMinerals,
        label: "Find Minerals",
        parent_skill: 186,
        art: object_icon(899.0, 931.0, 526.0, 558.0),
    },
];

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TrackingState {
    pub open: bool,
    pub enabled: [bool; SERVICES.len() + RESOURCE_FILTERS.len()],
    pub gathering_available: [bool; RESOURCE_FILTERS.len()],
}

impl TrackingState {
    pub fn toggle_action(&mut self, action: &str) -> bool {
        let index = action
            .strip_prefix("minimap:tracking:")
            .and_then(|index| index.parse::<usize>().ok());
        if let Some(index) = index
            && index >= SERVICES.len()
            && !self
                .gathering_available
                .get(index - SERVICES.len())
                .copied()
                .unwrap_or(false)
        {
            return false;
        }
        let Some(enabled) = index.and_then(|index| self.enabled.get_mut(index)) else {
            return false;
        };
        *enabled = !*enabled;
        true
    }
}

pub fn tracking_minimap_blips(
    view: &MinimapView,
    tracking: &TrackingState,
    units: impl IntoIterator<Item = (u64, NpcFlags, [f32; 2])>,
) -> Vec<MinimapBlip> {
    units
        .into_iter()
        .filter_map(|(unit, flags, position)| {
            let (_, service) = SERVICES.iter().enumerate().find(|(index, service)| {
                tracking.enabled[*index] && flags.contains(service.flag)
            })?;
            Some(MinimapBlip {
                unit,
                kind: BlipKind::Tracking {
                    filter: service.filter,
                },
                offset: view.blip_offset(position)?,
            })
        })
        .collect()
}

pub fn resource_tracking_minimap_blips(
    view: &MinimapView,
    tracking: &TrackingState,
    resources: impl IntoIterator<Item = (u64, shared::protocol::GatherableResource, [f32; 2])>,
) -> Vec<MinimapBlip> {
    resources
        .into_iter()
        .filter_map(|(unit, resource, position)| {
            let (_, filter) = RESOURCE_FILTERS
                .iter()
                .enumerate()
                .find(|(index, filter)| {
                    tracking.gathering_available[*index]
                        && tracking.enabled[SERVICES.len() + *index]
                        && resource.parent_skill == filter.parent_skill
                })?;
            Some(MinimapBlip {
                unit,
                kind: BlipKind::Tracking {
                    filter: filter.filter,
                },
                offset: view.blip_offset(position)?,
            })
        })
        .collect()
}

pub(super) fn art(filter: TrackingFilter) -> SheetArt {
    let index = filter as usize;
    if index < SERVICES.len() {
        SERVICES[index].art
    } else {
        RESOURCE_FILTERS[index - SERVICES.len()].art
    }
}

const MENU_WIDTH: f32 = 224.0;
const ROW_HEIGHT: f32 = 20.0;
const MENU_PAD: f32 = 8.0;

pub(super) fn menu(state: &TrackingState) -> Element {
    if !state.open {
        return Vec::new();
    }
    let mut rows: Element = SERVICES
        .iter()
        .enumerate()
        .flat_map(|(index, service)| tracking_choice(index, service.label, state.enabled[index]))
        .collect();
    let mut visible_count = SERVICES.len();
    for (index, filter) in RESOURCE_FILTERS.iter().enumerate() {
        if state.gathering_available[index] {
            rows.extend(tracking_choice_at(
                SERVICES.len() + index,
                visible_count,
                filter.label,
                state.enabled[SERVICES.len() + index],
            ));
            visible_count += 1;
        }
    }
    // MenuStyle1 bottom inset15 (MenuTemplates.xml); row extent20.
    let height = MENU_PAD + ROW_HEIGHT * visible_count as f32 + 15.0;
    rows.splice(
        0..0,
        crate::selection_list::menu_background(
            "MinimapTrackingMenuBackground".into(),
            (MENU_WIDTH, height),
        ),
    );
    rsx! { r#frame {
        name: "MinimapTrackingMenu", width: MENU_WIDTH, height,
        strata: FrameStrata::Dialog, mouse_enabled: true,
        pos_type: "absolute", left: 0.0, top: 24.0,
        {rows}
    } }
}

fn tracking_choice(index: usize, label: &str, checked: bool) -> Element {
    tracking_choice_at(index, index, label, checked)
}

fn tracking_choice_at(index: usize, row: usize, label: &str, checked: bool) -> Element {
    let action = format!("minimap:tracking:{index}");
    let name = format!("MinimapTrackingChoice{index}");
    let top = MENU_PAD + ROW_HEIGHT * row as f32;
    crate::menu_primitives::menu_element(
        crate::menu_primitives::ContextMenuItem {
            name: &name,
            label,
            action: &action,
        },
        crate::menu_primitives::MenuElementKind::Checkbox { checked },
        true,
        (MENU_PAD, top, MENU_WIDTH - MENU_PAD * 2.0, ROW_HEIGHT),
    )
}
