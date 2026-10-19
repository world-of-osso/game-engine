//! Retail learned-recipe context menu (RecipeList.lua60–78).
use super::ProfessionView;
use crate::menu_primitives::{ContextMenuItem, MenuElementKind, context_menu_description};
use ui_toolkit::widget_def::Element;

pub(super) fn favorite_menu(view: &ProfessionView) -> Element {
    let Some((spell, [left, top])) = view.favorite_menu else { return vec![]; };
    if view.page != super::ProfessionPage::Recipes || !view.book.snapshot.spells.contains(&spell) {
        return vec![];
    }
    let label = if view.book.snapshot.favorites.contains(&spell) { "Remove Favorite" } else { "Set Favorite" };
    let action = format!("profession:favorite:{spell}");
    context_menu_description("ProfessionsRecipeContextMenu", (left, top), 0.0, &[
        (ContextMenuItem { name: "ProfessionsRecipeFavoriteToggle", label, action: &action }, MenuElementKind::Button, true),
    ])
}
