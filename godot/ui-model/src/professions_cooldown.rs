//! Recipe cooldown display: `C_TradeSkillUI.GetRecipeCooldown` from the spell's own and
//! category clocks, and RecipeSchematicForm.lua's `COOLDOWN_REMAINING` text.

/// Whole seconds until the recipe is ready: the longer of its own and its category clock.
pub fn remaining_secs(_own: Option<f32>, _category: Option<f32>) -> Option<u32> {
    None
}

/// `COOLDOWN_REMAINING` followed by the schematic's two-unit SecondsFormatter output.
pub fn remaining_text(_secs: u32) -> String {
    String::new()
}
