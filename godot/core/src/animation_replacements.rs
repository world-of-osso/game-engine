//! Aura312's local Retail AnimReplacementSet / AnimReplacement exports.
use std::{collections::HashMap, path::Path};

use crate::csv_util::read_numeric_rows;

#[derive(Default)]
pub struct AnimationReplacementCatalog {
    sets: HashMap<u32, (u8, Vec<Replacement>)>,
}

struct Replacement {
    source: u16,
    destination: u16,
    conditional_flags: u32,
}

fn read_rows<const N: usize>(path: &Path, columns: [&str; N]) -> Result<Vec<[i64; N]>, String> {
    let mut rows = Vec::new();
    read_numeric_rows(path, columns, |row| rows.push(row))?;
    Ok(rows)
}

fn narrow<T: TryFrom<i64>>(value: i64, column: &str) -> Result<T, String> {
    T::try_from(value).map_err(|_| format!("AnimReplacement {column} out of range: {value}"))
}

impl AnimationReplacementCatalog {
    pub fn load(root: &Path) -> Result<Self, String> {
        let mut catalog = Self::default();
        for [id, order] in read_rows(&root.join("AnimReplacementSet.csv"), ["ID", "ExecOrder"])? {
            catalog
                .sets
                .insert(narrow(id, "ID")?, (narrow(order, "ExecOrder")?, Vec::new()));
        }
        let columns = [
            "SrcAnimID",
            "DstAnimID",
            "ConditionalFlags",
            "ParentAnimReplacementSetID",
        ];
        for [source, destination, flags, parent] in
            read_rows(&root.join("AnimReplacement.csv"), columns)?
        {
            let id = narrow(parent, "ParentAnimReplacementSetID")?;
            let set = catalog
                .sets
                .get_mut(&id)
                .ok_or_else(|| format!("AnimReplacement references missing set {id}"))?;
            set.1.push(Replacement {
                source: narrow(source, "SrcAnimID")?,
                destination: narrow(destination, "DstAnimID")?,
                conditional_flags: narrow(flags, "ConditionalFlags")?,
            });
        }
        Ok(catalog)
    }

    /// Compose active sets in their authored execution order; replacement is one-hop.
    pub fn replacements(&self, ids: &[u32]) -> Result<HashMap<u16, u16>, String> {
        let mut active = ids
            .iter()
            .map(|id| {
                self.sets
                    .get(id)
                    .map(|set| (*id, set))
                    .ok_or_else(|| format!("Missing AnimReplacementSet {id}"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        active.sort_by_key(|(id, (order, _))| (*order, *id));
        let mut map = HashMap::new();
        for (id, (_, rows)) in active {
            for row in rows {
                if row.conditional_flags != 0 {
                    return Err(format!(
                        "AnimReplacementSet {id}: unsupported ConditionalFlags {}",
                        row.conditional_flags
                    ));
                }
                map.insert(row.source, row.destination);
            }
        }
        Ok(map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aura312_matrix_rows_load_from_local_retail_exports() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let catalog = AnimationReplacementCatalog::load(&root).unwrap();
        let stand = catalog.replacements(&[536]).unwrap();
        assert_eq!(stand.get(&0), Some(&25)); // 2463, Metamorphosis187827.
        let run = catalog.replacements(&[499]).unwrap();
        assert_eq!(run.get(&5), Some(&223)); // 2349, Spectral Sight1251417.
        assert!(catalog.replacements(&[]).unwrap().is_empty());
        let spatial = catalog.replacements(&[1013]).unwrap();
        assert_eq!(spatial.get(&37), Some(&1604)); // 3874, Spatial Paradox406732.
    }
}
