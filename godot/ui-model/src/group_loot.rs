//! Retail group-roll prompts and authoritative loot history. No skin-specific behavior.
use shared::loot::LootRollChoice;
use shared::protocol::{GroupLootChoice, GroupLootRoll, LootContent};

pub const VISIBLE_ROLLS: usize = 4;
const HISTORY_LIMIT: usize = 50;

#[derive(Default)]
pub struct GroupLootState {
    rolls: Vec<GroupLootRoll>,
    pub history_open: bool,
}

impl GroupLootState {
    pub fn apply(&mut self, snapshot: GroupLootRoll) -> Vec<String> {
        let previous = self
            .rolls
            .iter()
            .position(|roll| roll.roll_id == snapshot.roll_id);
        let lines = previous.map_or_else(Vec::new, |index| {
            chat_changes(&self.rolls[index], &snapshot)
        });
        if let Some(index) = previous {
            self.rolls[index] = snapshot;
        } else {
            self.rolls.push(snapshot);
        }
        while self.rolls.len() > HISTORY_LIMIT {
            let Some(index) = self.rolls.iter().position(|roll| roll.complete) else {
                break;
            };
            self.rolls.remove(index);
        }
        lines
    }

    pub fn tick(&mut self, delta: f32) {
        for roll in self.rolls.iter_mut().filter(|roll| !roll.complete) {
            roll.seconds_remaining = (roll.seconds_remaining - delta).max(0.0);
        }
    }

    pub fn visible(&self, viewer: u64) -> Vec<&GroupLootRoll> {
        self.rolls
            .iter()
            .filter(|roll| pending_for(roll, viewer))
            .take(VISIBLE_ROLLS)
            .collect()
    }

    pub fn history(&self) -> &[GroupLootRoll] {
        &self.rolls
    }

    pub fn request(
        &self,
        viewer: u64,
        roll_id: u64,
        choice: LootRollChoice,
    ) -> Option<GroupLootChoice> {
        let roll = self.rolls.iter().find(|roll| roll.roll_id == roll_id)?;
        if !pending_for(roll, viewer) {
            return None;
        }
        let player = roll
            .players
            .iter()
            .find(|player| player.character_id == viewer)?;
        let eligible = match choice {
            LootRollChoice::Need => player.can_need,
            LootRollChoice::Transmog => player.can_transmog,
            LootRollChoice::Greed => !player.can_transmog,
            LootRollChoice::Pass => true,
        };
        eligible.then_some(GroupLootChoice { roll_id, choice })
    }
}

fn pending_for(roll: &GroupLootRoll, viewer: u64) -> bool {
    !roll.complete
        && roll.seconds_remaining > 0.0
        && roll
            .players
            .iter()
            .any(|player| player.character_id == viewer && player.response.is_none())
}

pub fn choice_name(choice: LootRollChoice) -> &'static str {
    match choice {
        LootRollChoice::Need => "Need",
        LootRollChoice::Greed => "Greed",
        LootRollChoice::Transmog => "Transmogrification",
        LootRollChoice::Pass => "Pass",
    }
}

pub fn item_name(roll: &GroupLootRoll) -> &str {
    match &roll.item {
        LootContent::Item { name, .. } => name,
        LootContent::Money { .. } => "",
    }
}

pub fn result_text(roll: &GroupLootRoll) -> String {
    if !roll.complete {
        return "Waiting for rolls".into();
    }
    let winner = roll.winner.and_then(|winner| {
        roll.players
            .iter()
            .find(|player| player.character_id == winner)
    });
    match winner {
        Some(winner) => format!("{} won: {}", winner.name, item_name(roll)), // LOOT_ROLL_WON
        None => format!("[Loot]: Everyone passed on: {}", item_name(roll)),  // LOOT_ROLL_ALL_PASSED
    }
}

fn chat_changes(previous: &GroupLootRoll, current: &GroupLootRoll) -> Vec<String> {
    let mut lines = Vec::new();
    for player in &current.players {
        let before = previous
            .players
            .iter()
            .find(|before| before.character_id == player.character_id);
        let Some(response) = player
            .response
            .filter(|response| before.is_some_and(|before| before.response != Some(*response)))
        else {
            continue;
        };
        let line = if response.choice == LootRollChoice::Pass {
            format!("{} passed on: {}", player.name, item_name(current))
        } else {
            format!(
                "{} has selected {} for: {}",
                player.name,
                choice_name(response.choice),
                item_name(current)
            )
        };
        lines.push(line);
    }
    if current.complete && !previous.complete {
        // GlobalStrings LOOT_ROLL_ROLLED_NEED/GREED: announce server RNG only
        // on resolution, not on selection or history replay.
        for player in &current.players {
            if let Some(response) = player.response
                && response.choice != LootRollChoice::Pass
            {
                lines.push(format!(
                    "{} Roll - {} for {} by {}",
                    choice_name(response.choice),
                    response.roll_value,
                    item_name(current),
                    player.name
                ));
            }
        }
        lines.push(result_text(current));
    }
    lines
}

pub fn parse_choice(action: &str) -> Option<(u64, LootRollChoice)> {
    let (id, choice) = action.strip_prefix("group_loot:")?.split_once(':')?;
    let choice = match choice {
        "need" => LootRollChoice::Need,
        "greed" => LootRollChoice::Greed,
        "transmog" => LootRollChoice::Transmog,
        "pass" => LootRollChoice::Pass,
        _ => return None,
    };
    Some((id.parse().ok()?, choice))
}

#[cfg(test)]
#[path = "group_loot_tests.rs"]
mod tests;
