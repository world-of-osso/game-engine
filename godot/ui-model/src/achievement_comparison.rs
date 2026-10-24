//! Original comparison-page cache updates; rendering lives in ui-achievements.
use super::*;

pub(super) fn cache_page(comparison: &mut AchievementComparison, page: &AchievementCatalogPage) {
    match page {
        AchievementCatalogPage::Achievement {
            achievement_id,
            entry,
        } => {
            comparison.entries.insert(*achievement_id, entry.clone());
        }
        AchievementCatalogPage::Category { achievements, .. }
        | AchievementCatalogPage::Statistics { achievements, .. }
        | AchievementCatalogPage::Search { achievements, .. } => {
            comparison.entries.extend(
                achievements
                    .iter()
                    .map(|entry| (entry.achievement_id, Some(entry.clone()))),
            );
        }
        AchievementCatalogPage::Criteria {
            achievement_id,
            criteria,
            next_id,
        } => {
            if let Some(Some(entry)) = comparison.entries.get_mut(achievement_id) {
                merge_criteria(entry, criteria, *next_id);
            }
        }
        _ => {}
    }
}

fn merge_criteria(
    entry: &mut AchievementCatalogEntry,
    criteria: &[AchievementCriterionLine],
    next: Option<u32>,
) {
    for line in criteria {
        if let Some(old) = entry
            .criteria
            .iter_mut()
            .find(|old| old.tree_id == line.tree_id)
        {
            *old = line.clone();
        } else {
            entry.criteria.push(line.clone());
        }
    }
    entry.next_criteria_id = next;
}
