//! Offline PlayerSpellsFrame capture through the production spellbook projection.
use super::{RegistryUi, party_preview};
use game_engine_ui_model::spellbook_frame_component::PlayerSpellsTab;
use game_engine_ui_model::spellbook_preview::{load_class_preview_state, load_preview_state};
use godot::prelude::*;
use ui_toolkit::atlas::{ActiveSkin, set_thread_skin};

#[godot_api(secondary)]
impl RegistryUi {
    #[func]
    fn show_spellbook_preview(&mut self) -> GString {
        self.show_spellbook_preview_skin(ActiveSkin::Modern)
    }

    #[func]
    fn show_forever_spellbook_preview(&mut self) -> GString {
        self.show_spellbook_preview_skin(ActiveSkin::Forever)
    }

    /// Offline native pointer hit and the same shared spell GameTooltip builder as live UI.
    #[func]
    fn update_spellbook_preview_tooltip(&self, mut host: Gd<RegistryUi>, at: Vector2) -> GString {
        let result = self.spellbook_preview_tooltip_at(at).and_then(|view| {
            let mut host = host.bind_mut();
            host.set_ui_scale(1.0)?;
            if host.registry().is_some() {
                host.set_state(view)
            } else {
                host.show_game_tooltip(view)
            }
        });
        GString::from(result.err().unwrap_or_default().as_str())
    }

    fn show_spellbook_preview_skin(&mut self, skin: ActiveSkin) -> GString {
        let result = party_preview::load_data_root().and_then(|()| {
            set_thread_skin(skin);
            self.set_ui_scale(1.0)?;
            let page = std::env::var("GODOT_SPELLBOOK_TAB").unwrap_or_default();
            let tab = match page.as_str() {
                "" | "spellbook" => PlayerSpellsTab::Spellbook,
                "specialization" => PlayerSpellsTab::Specialization,
                "talents" => PlayerSpellsTab::Talents,
                _ => return Err(format!("Unknown offline PlayerSpells page: {page}")),
            };
            let data = game_engine_ui_model::paths::resolve_data_path("");
            let mut state = match read_preview_selection()? {
                Some((class, spec)) => load_class_preview_state(&data, tab, class, spec)?,
                None => load_preview_state(&data, tab)?,
            };
            if tab == PlayerSpellsTab::Talents
                && std::env::var("GODOT_TALENT_PENDING").as_deref() == Ok("1")
            {
                game_engine_ui_model::spellbook_preview::stage_talent_preview(&mut state)?;
                let view = state
                    .talents
                    .as_ref()
                    .ok_or("Pending preview lost talent view")?;
                godot_print!(
                    "TALENT_PENDING {:?} POINTS {:?}",
                    view.editor.apply(),
                    view.editor.unspent(view)
                );
            }
            let size = self
                .base()
                .get_viewport()
                .ok_or("Spellbook preview has no viewport")?
                .get_visible_rect()
                .size;
            state.viewport = [size.x, size.y];
            self.show_spellbook(state)
        });
        GString::from(result.err().unwrap_or_default().as_str())
    }
}

impl RegistryUi {
    fn spellbook_preview_tooltip_at(
        &self,
        at: Vector2,
    ) -> Result<game_engine_ui_model::game_tooltip::GameTooltipView, String> {
        use game_engine_ui_model::game_tooltip::{
            GameTooltipView, OwnerSide, TooltipScreen, place,
        };
        let hit = self.pointer_frame_at(at).and_then(|id| {
            crate::tooltips::named_ancestor(self.registry()?, id, |frame| {
                game_engine_ui_model::talents::talent_button_spell(frame.name.as_deref()?)
            })
        });
        let Some((owner, spell)) = hit else {
            return Ok(GameTooltipView::default());
        };
        let rect = self
            .frame_id_rect(owner)
            .ok_or("Talents preview tooltip owner vanished")?;
        let data = game_engine_ui_model::paths::resolve_data_path("");
        let tooltip = load_talent_preview_tooltip(&data, spell)?.owned(
            [rect.position.x, rect.position.y, rect.size.x, rect.size.y],
            OwnerSide::Right,
        );
        let size = self
            .base()
            .get_viewport()
            .ok_or("Talents preview lacks viewport")?
            .get_visible_rect()
            .size;
        Ok(GameTooltipView {
            main: place(
                tooltip.for_skin(ui_toolkit::atlas::thread_skin()),
                TooltipScreen {
                    size: [size.x, size.y],
                    cursor: [at.x, at.y],
                },
            ),
            ..Default::default()
        })
    }
}
fn read_preview_selection() -> Result<Option<(u32, u32)>, String> {
    let class = std::env::var("GODOT_PREVIEW_CLASS").ok();
    let spec = std::env::var("GODOT_PREVIEW_SPEC").ok();
    match (class, spec) {
        (None, None) => Ok(None),
        (Some(class), Some(spec)) => {
            let class = class
                .parse()
                .map_err(|error| format!("GODOT_PREVIEW_CLASS: {error}"))?;
            let spec = spec
                .parse()
                .map_err(|error| format!("GODOT_PREVIEW_SPEC: {error}"))?;
            Ok(Some((class, spec)))
        }
        _ => Err("Set GODOT_PREVIEW_CLASS and GODOT_PREVIEW_SPEC together".into()),
    }
}

fn load_talent_preview_tooltip(
    data: &std::path::Path,
    id: u32,
) -> Result<game_engine_ui_model::game_tooltip::GameTooltip, String> {
    use game_engine_core::spell_catalog::{
        SpellCatalogPaths, SpellTextContext, load_spell_catalog,
    };
    use game_engine_ui_model::game_tooltip::spell::{SpellTooltipInput, spell_tooltip};
    let catalog = load_spell_catalog(&SpellCatalogPaths::for_data_dir(data))?;
    let spell = catalog
        .get(id)
        .ok_or_else(|| format!("Talents preview lacks local spell {id}"))?;
    let context = SpellTextContext {
        spec_id: Some(read_preview_selection()?.map_or(62, |(_, spec)| spec)),
        ..Default::default()
    };
    let description = catalog
        .render_description(id, &context)
        .ok_or_else(|| format!("Talents preview lacks description for {id}"))?;
    Ok(spell_tooltip(
        spell,
        &SpellTooltipInput {
            description,
            ..Default::default()
        },
    ))
}
