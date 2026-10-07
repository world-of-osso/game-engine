//! Client quest state driven by the server quest runtime (`shared::protocol` quest and
//! NPC interaction messages): the quest log, the watch list, per-NPC quest giver
//! markers and the open quest giver dialog (Retail `GossipFrame` / `QuestFrame`).
//!
//! Reducers are pure methods; the networking systems feed messages in and post the
//! returned Retail system lines to chat.

use std::collections::{HashMap, HashSet};

use shared::protocol::{
    GossipMenu, GossipMenuOption, NpcRole, QuestEntrySnapshot, QuestFailedReason,
    QuestGiverOfferReward, QuestGiverQuestComplete, QuestGiverQuestDetails, QuestGiverQuestEntry,
    QuestGiverQuestList, QuestGiverRequestItems, QuestGiverStatus, QuestLogSnapshot,
    QuestLogUpdate, QuestMarkerClass, QuestPoiSnapshot,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuestNotice {
    System(String),
    Progress(String),
}

impl std::fmt::Display for QuestNotice {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            Self::System(text) | Self::Progress(text) => text,
        };
        formatter.write_str(text)
    }
}

#[derive(Default, Debug, Clone, PartialEq)]
pub struct QuestRuntime {
    /// Quest log in server order.
    pub log: Vec<QuestEntrySnapshot>,
    /// Watched quest ids in server order (the objective tracker order).
    pub watched: Vec<u32>,
    /// Quest giver marker per NPC (server entity bits).
    pub giver_status: HashMap<u64, QuestGiverStatus>,
    pub dialog: Option<QuestDialog>,
}

/// A role frame (auction house, vendor, ...) the server opened for an NPC interaction,
/// or the end of that interaction. Frames other than the quest dialog read these.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NpcFrameEvent {
    Opened { npc: u64, role: NpcRole },
    Closed { npc: u64 },
}

/// A player action for the server quest / interaction runtime.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NpcInteractionRequest {
    Hello {
        npc: u64,
    },
    SelectGossip {
        npc: u64,
        option_id: u32,
    },
    QueryQuest {
        npc: u64,
        quest_id: u32,
    },
    Accept {
        npc: u64,
        quest_id: u32,
    },
    Complete {
        npc: u64,
        quest_id: u32,
    },
    ChooseReward {
        npc: u64,
        quest_id: u32,
        choice: Option<u8>,
    },
    Abandon {
        quest_id: u32,
    },
    SetWatched {
        quest_id: u32,
        watched: bool,
    },
    Close {
        npc: u64,
    },
}

/// Player-facing UI state that the server does not own.
#[derive(Default, Debug, Clone, PartialEq)]
pub struct QuestUiState {
    pub log_selected: Option<u32>,
    /// Collapsed quest log headers (`sort_id`).
    pub collapsed_headers: HashSet<i32>,
    /// Objective tracker minimized to its header.
    pub tracker_collapsed: bool,
    /// Quests module of the tracker minimized to its header.
    pub quests_collapsed: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct QuestDialog {
    pub npc: u64,
    pub npc_name: String,
    pub page: QuestDialogPage,
}

#[derive(Debug, Clone, PartialEq)]
pub enum QuestDialogPage {
    /// Gossip greeting: NPC text, gossip options and the giver's quest list.
    Greeting {
        text: String,
        options: Vec<GossipMenuOption>,
        quests: Vec<QuestGiverQuestEntry>,
    },
    Detail(QuestGiverQuestDetails),
    Progress(QuestGiverRequestItems),
    Reward {
        offer: QuestGiverOfferReward,
        choice: Option<u8>,
    },
}

fn objective_progress_notices(
    previous: &QuestEntrySnapshot,
    changed: &QuestEntrySnapshot,
) -> Vec<QuestNotice> {
    previous
        .objectives
        .iter()
        .zip(&changed.objectives)
        .filter(|(before, after)| {
            let same_objective = before.kind == after.kind && before.object_id == after.object_id;
            same_objective && after.current > before.current
        })
        .map(|(_, objective)| {
            QuestNotice::Progress(format!(
                "{}: {}/{}",
                objective.text, objective.current, objective.required,
            ))
        })
        .collect()
}

impl QuestRuntime {
    pub fn entry(&self, quest_id: u32) -> Option<&QuestEntrySnapshot> {
        self.log.iter().find(|entry| entry.quest_id == quest_id)
    }

    pub fn is_watched(&self, quest_id: u32) -> bool {
        self.watched.contains(&quest_id)
    }

    /// Watched quests in tracker order.
    pub fn watched_entries(&self) -> Vec<&QuestEntrySnapshot> {
        self.watched
            .iter()
            .filter_map(|id| self.entry(*id))
            .collect()
    }

    /// Objective areas (`QuestPOI` blobs) of the watched quests: the polygons of each
    /// unfinished objective; finished quests show their turn-in pin instead.
    pub fn watched_objective_areas(&self) -> Vec<&QuestPoiSnapshot> {
        self.watched_entries()
            .into_iter()
            .filter(|entry| !entry.completed)
            .flat_map(|entry| {
                entry.pois.iter().filter(move |poi| {
                    let objective = usize::try_from(poi.objective_index)
                        .ok()
                        .and_then(|index| entry.objectives.get(index));
                    poi.points.len() >= 3 && objective.is_some_and(|objective| !objective.completed)
                })
            })
            .collect()
    }

    pub fn apply_snapshot(&mut self, snapshot: QuestLogSnapshot) {
        self.log = snapshot.entries;
        self.watched = snapshot.watched_quest_ids;
    }

    /// Applies a log delta; newly added quests produce `ERR_QUEST_ACCEPTED_S`.
    pub fn apply_update(&mut self, update: QuestLogUpdate) -> Vec<QuestNotice> {
        let mut notices = Vec::new();
        self.log
            .retain(|entry| !update.removed.contains(&entry.quest_id));
        for changed in update.changed {
            match self
                .log
                .iter_mut()
                .find(|entry| entry.quest_id == changed.quest_id)
            {
                Some(entry) => {
                    notices.extend(objective_progress_notices(entry, &changed));
                    *entry = changed;
                }
                None => {
                    notices.push(QuestNotice::System(format!(
                        "Quest accepted: {}",
                        changed.title
                    )));
                    self.log.push(changed);
                }
            }
        }
        self.watched = update.watched_quest_ids;
        notices
    }

    pub fn apply_statuses(&mut self, statuses: impl IntoIterator<Item = (u64, QuestGiverStatus)>) {
        for (npc, status) in statuses {
            self.giver_status.insert(npc, status);
        }
    }

    pub fn open_gossip(&mut self, npc: u64, npc_name: String, menu: GossipMenu) {
        self.dialog = Some(QuestDialog {
            npc,
            npc_name,
            page: QuestDialogPage::Greeting {
                text: menu.text,
                options: menu.options,
                quests: Vec::new(),
            },
        });
    }

    /// A quest giver role opened without a gossip menu: an empty greeting that the
    /// quest list fills.
    pub fn open_quest_giver(&mut self, npc: u64, npc_name: String) {
        self.open_gossip(
            npc,
            npc_name,
            GossipMenu {
                menu_id: 0,
                text: String::new(),
                options: Vec::new(),
            },
        );
    }

    pub fn apply_quest_list(&mut self, list: QuestGiverQuestList) {
        let Some(dialog) = self.dialog.as_mut().filter(|dialog| dialog.npc == list.npc) else {
            return;
        };
        if let QuestDialogPage::Greeting { quests, .. } = &mut dialog.page {
            *quests = list.quests;
        }
    }

    pub fn show_details(&mut self, npc_name: String, details: QuestGiverQuestDetails) {
        let npc = details.npc;
        self.show_page(npc, npc_name, QuestDialogPage::Detail(details));
    }

    pub fn show_progress(&mut self, npc_name: String, request: QuestGiverRequestItems) {
        let npc = request.npc;
        self.show_page(npc, npc_name, QuestDialogPage::Progress(request));
    }

    pub fn show_reward(&mut self, npc_name: String, offer: QuestGiverOfferReward) {
        let npc = offer.npc;
        self.show_page(
            npc,
            npc_name,
            QuestDialogPage::Reward {
                offer,
                choice: None,
            },
        );
    }

    fn show_page(&mut self, npc: u64, npc_name: String, page: QuestDialogPage) {
        self.dialog = Some(QuestDialog {
            npc,
            npc_name,
            page,
        });
    }

    /// Selects reward choice `index` on the open reward page.
    pub fn choose_reward(&mut self, index: u8) {
        if let Some(QuestDialog {
            page: QuestDialogPage::Reward { offer, choice },
            ..
        }) = self.dialog.as_mut()
            && usize::from(index) < offer.rewards.choice_items.len()
        {
            *choice = Some(index);
        }
    }

    /// Closes the dialog when it belongs to `npc`; returns whether one closed.
    pub fn close_dialog_for(&mut self, npc: u64) -> bool {
        if self.dialog.as_ref().is_some_and(|dialog| dialog.npc == npc) {
            self.dialog = None;
            return true;
        }
        false
    }

    /// A turn-in finished: Retail `ERR_QUEST_COMPLETE_S`, `ERR_QUEST_REWARD_EXP_I` and
    /// `ERR_QUEST_REWARD_MONEY_S` lines; that quest's reward page closes.
    pub fn complete_quest(&mut self, complete: &QuestGiverQuestComplete) -> Vec<String> {
        let title = self
            .completed_title(complete.quest_id)
            .unwrap_or_else(|| format!("Quest {}", complete.quest_id));
        if self.reward_page_for(complete.quest_id) {
            self.dialog = None;
        }
        let mut notices = vec![format!("{title} completed.")];
        if complete.xp > 0 {
            notices.push(format!("Experience gained: {}.", complete.xp));
        }
        if complete.money > 0 {
            notices.push(format!("Received {}.", money_text(complete.money)));
        }
        for item in &complete.items {
            notices.push(received_item_text(&item.name, item.count));
        }
        notices
    }

    fn completed_title(&self, quest_id: u32) -> Option<String> {
        match &self.dialog {
            Some(QuestDialog {
                page: QuestDialogPage::Reward { offer, .. },
                ..
            }) if offer.quest_id == quest_id => Some(offer.title.clone()),
            _ => self.entry(quest_id).map(|entry| entry.title.clone()),
        }
    }

    fn reward_page_for(&self, quest_id: u32) -> bool {
        matches!(
            &self.dialog,
            Some(QuestDialog {
                page: QuestDialogPage::Reward { offer, .. },
                ..
            }) if offer.quest_id == quest_id
        )
    }
}

/// `interface/buttons/talktome.m2`: yellow `!`.
pub const TALKTOME_AVAILABLE_FDID: u32 = 130_731;
/// `interface/buttons/talktomequestionmark.m2`: yellow `?`.
pub const TALKTOME_TURN_IN_FDID: u32 = 130_738;
/// `interface/buttons/talktomegrey.m2`: grey `!`.
pub const TALKTOME_UNAVAILABLE_FDID: u32 = 130_734;
/// `interface/buttons/talktomequestion_grey.m2`: grey `?`.
pub const TALKTOME_INCOMPLETE_FDID: u32 = 130_735;

/// The `talktome` marker M2 floating over a quest giver with `status`, by the
/// `interface/buttons/talktome*` names of each kind in the community listfile. Trivial
/// quests show none: Retail's "Trivial Quests" tracking defaults to off
/// (`PROXY_TRIVIAL_QUEST_FILTERING`, `InterfaceOverrides.lua:296`, `Settings.Default.False`).
/// An unfinished calling wears the campaign `?`, as `QuestUtil.GetQuestIconActive` gives
/// both `CampaignInProgressQuestIcon` (`QuestUtils.lua:186`).
pub fn quest_marker_model(status: QuestGiverStatus) -> Option<u32> {
    use QuestMarkerClass::*;
    let fdid = match status {
        QuestGiverStatus::None | QuestGiverStatus::Trivial(_) => return None,
        QuestGiverStatus::Future(class) => match class {
            Important => 5_390_225, // talktome_important_locked.m2
            Campaign => 5_390_226,  // talktome_journey_locked.m2
            Legendary => 5_390_227, // talktome_legendary_locked.m2
            Normal | Repeatable | Meta | Calling => TALKTOME_UNAVAILABLE_FDID,
        },
        QuestGiverStatus::Incomplete(class) => match class {
            Normal => TALKTOME_INCOMPLETE_FDID,
            Repeatable => 5_374_198, // talktomequestion_repeatable_grey.m2
            Meta => 5_374_194,       // talktomequestion_wrapper_grey.m2
            Calling | Campaign => 3_486_410, // talktomequestion_journey_grey.m2
            Legendary => 5_152_919,  // talktome_new_questionlegendary_grey.m2
            Important => 5_217_341,  // talktomequestion_important_grey.m2
        },
        QuestGiverStatus::Available(class) => match class {
            Normal => TALKTOME_AVAILABLE_FDID,
            Repeatable => 5_374_195, // talktome_repeatable.m2
            Meta => 5_374_191,       // talktome_wrapper.m2
            Calling => 3_605_006,    // talktome_callings.m2
            Campaign => 3_486_398,   // talktome_journey.m2
            Legendary => 3_489_738,  // talktomeorange_new.m2
            Important => 5_217_338,  // talktome_important.m2
        },
        QuestGiverStatus::Reward(class) => match class {
            Normal => TALKTOME_TURN_IN_FDID,
            Repeatable => 5_374_197, // talktomequestion_repeatable.m2
            Meta => 5_374_193,       // talktomequestion_wrapper.m2
            Calling => 3_753_369,    // talktome_callingsquestion.m2
            Campaign => 3_486_399,   // talktomequestion_journey.m2
            Legendary => 3_491_680,  // talktome_new_questionlegendary.m2
            Important => 5_217_340,  // talktomequestion_important.m2
        },
    };
    Some(fdid)
}

/// `LOOT_ITEM_PUSHED_SELF` / `LOOT_ITEM_PUSHED_SELF_MULTIPLE`.
fn received_item_text(name: &str, count: u32) -> String {
    if count > 1 {
        format!("You receive item: [{name}]x{count}.")
    } else {
        format!("You receive item: [{name}].")
    }
}

/// Retail `GetMoneyString` without icons: "1 Gold 5 Silver 3 Copper".
pub fn money_text(copper: u32) -> String {
    let (gold, silver, copper) = (copper / 10_000, copper / 100 % 100, copper % 100);
    let parts: Vec<String> = [(gold, "Gold"), (silver, "Silver"), (copper, "Copper")]
        .into_iter()
        .filter(|(amount, _)| *amount > 0)
        .map(|(amount, unit)| format!("{amount} {unit}"))
        .collect();
    if parts.is_empty() {
        "0 Copper".into()
    } else {
        parts.join(" ")
    }
}

/// Retail quest failure text (`ERR_QUEST_*` / `QuestFailedReason` strings).
pub fn quest_failed_text(reason: QuestFailedReason) -> &'static str {
    match reason {
        QuestFailedReason::NotAvailable => "That quest is not available to you.",
        QuestFailedReason::TooFar => "You are too far away.",
        QuestFailedReason::LowLevel => "You are not high enough level for that quest.",
        QuestFailedReason::HighLevel => "You are too high level for that quest.",
        QuestFailedReason::WrongClass => "That quest is not available to your class.",
        QuestFailedReason::WrongRace => "That quest is not available to your race.",
        QuestFailedReason::DontHaveRequirement => "You don't meet the requirements for that quest.",
        QuestFailedReason::AlreadyOn => "You are already on that quest.",
        QuestFailedReason::AlreadyDone => "You have completed that quest.",
        QuestFailedReason::QuestLogFull => "Your quest log is full.",
        QuestFailedReason::NotInLog => "That quest is not in your quest log.",
        QuestFailedReason::ObjectivesIncomplete => {
            "You don't have the required items with you.  Check storage."
        }
        QuestFailedReason::NotEnoughMoney => "You don't have enough money for that quest.",
        QuestFailedReason::InventoryFull => "Inventory is full.",
        QuestFailedReason::InvalidRewardChoice => "You must choose a reward.",
    }
}

/// Who the quest text addresses (`$N`, `$C`, `$R`, `$G`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct QuestTextTokens {
    pub name: String,
    pub class: String,
    pub race: String,
    pub female: bool,
}

/// Substitutes Retail quest text tokens: `$N`/`$n` name, `$C`/`$c` class,
/// `$R`/`$r` race (lower case for the lower-case token), `$B`/`$b` line break and
/// `$Gmale:female;` gendered words, then English `|5` indefinite articles (`|5^` capitalized).
pub fn substitute_quest_text(text: &str, tokens: &QuestTextTokens) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '$' {
            out.push(c);
            continue;
        }
        let Some(&token) = chars.peek() else {
            out.push(c);
            break;
        };
        let replacement = match token {
            'N' | 'n' => Some(tokens.name.clone()),
            'C' => Some(tokens.class.clone()),
            'c' => Some(tokens.class.to_lowercase()),
            'R' => Some(tokens.race.clone()),
            'r' => Some(tokens.race.to_lowercase()),
            'B' | 'b' => Some("\n".to_string()),
            _ => None,
        };
        if let Some(replacement) = replacement {
            chars.next();
            out.push_str(&replacement);
        } else if token == 'G' || token == 'g' {
            chars.next();
            out.push_str(&gendered_word(&mut chars, tokens.female));
        } else {
            out.push(c);
        }
    }
    resolve_indefinite_articles(&out)
}

/// Retail FontStrings resolve `|5word` to `a word` / `an word` after player substitution.
fn resolve_indefinite_articles(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '|' {
            out.push(ch);
            continue;
        }
        match chars.peek() {
            Some('|') => {
                chars.next();
                out.push_str("||");
            }
            Some('5') => {
                chars.next();
                out.push_str(read_indefinite_article(&mut chars));
            }
            _ => out.push(ch),
        }
    }
    out
}

fn read_indefinite_article(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> &'static str {
    let capitalized = chars.peek() == Some(&'^');
    if capitalized {
        chars.next();
    }
    while chars.peek() == Some(&' ') {
        chars.next();
    }
    let vowel = chars.peek().is_some_and(|ch| "aeiouAEIOU".contains(*ch));
    match (capitalized, vowel) {
        (false, false) => "a ",
        (false, true) => "an ",
        (true, false) => "A ",
        (true, true) => "An ",
    }
}

/// Reads `male:female;` after `$G` and returns the word for the player's sex.
fn gendered_word(chars: &mut std::iter::Peekable<std::str::Chars<'_>>, female: bool) -> String {
    let body: String = chars.by_ref().take_while(|c| *c != ';').collect();
    let (male_word, female_word) = body.split_once(':').unwrap_or((body.as_str(), ""));
    let word = if female { female_word } else { male_word };
    word.trim().to_string()
}

#[cfg(test)]
#[path = "quest_runtime_tests.rs"]
mod tests;
