//! Only ModifierTree nodes reached by authored CooldownSet PlayerConditions.
//! TrinityCore a352b1fa CriteriaHandler::ModifierTreeSatisfied/ModifierSatisfied:
//! operators2/3/4/8; PlayerHasAura8 and PlayerKnowsSpell104.
use crate::cooldown_conditions::ConditionActor;
use crate::spell_catalog::csv_records::CsvTable;
use std::collections::BTreeMap;
use std::path::Path;

type ModifierRows = BTreeMap<u32, [i64; 6]>;
type ModifierChildren = BTreeMap<u32, Vec<u32>>;
const MODIFIER_FIELDS: [&str; 8] = [
    "ID",
    "Parent",
    "Operator",
    "Amount",
    "Type",
    "Asset",
    "SecondaryAsset",
    "TertiaryAsset",
];

#[derive(Clone, Debug)]
enum Predicate {
    False,
    KnownSpell(u32),
    Aura(u32),
    Not(Box<Self>),
    All(Vec<Self>),
    Some(u32, Vec<Self>),
}

impl Predicate {
    fn evaluate(&self, actor: &ConditionActor<'_>) -> bool {
        match self {
            Self::False => false,
            Self::KnownSpell(spell) => actor.known.contains(spell),
            Self::Aura(spell) => actor.auras.contains_key(spell),
            Self::Not(child) => !child.evaluate(actor),
            Self::All(children) => children.iter().all(|child| child.evaluate(actor)),
            Self::Some(amount, children) => {
                children
                    .iter()
                    .filter(|child| child.evaluate(actor))
                    .take(*amount as usize)
                    .count()
                    == *amount as usize
            }
        }
    }
}

#[derive(Clone, Debug, Default)]
pub(super) struct CooldownModifiers(BTreeMap<u32, Predicate>);

impl CooldownModifiers {
    pub(super) fn load(dir: &Path, roots: &[u32]) -> Result<Self, String> {
        if roots.is_empty() {
            return Ok(Self::default());
        }
        let (rows, children) = read_modifier_index(dir)?;
        let trees = roots
            .iter()
            .map(|id| Ok((*id, build_predicate(*id, &rows, &children)?)))
            .collect::<Result<_, String>>()?;
        Ok(Self(trees))
    }

    pub(super) fn evaluate(&self, id: u32, actor: &ConditionActor<'_>) -> Result<bool, String> {
        if id == 0 {
            return Ok(true);
        }
        self.0
            .get(&id)
            .map(|tree| tree.evaluate(actor))
            .ok_or_else(|| format!("ModifierTree {id}: not loaded"))
    }
}

fn read_modifier_index(dir: &Path) -> Result<(ModifierRows, ModifierChildren), String> {
    let table = CsvTable::read(&dir.join("ModifierTree.csv"))?;
    let columns = MODIFIER_FIELDS
        .map(|name| table.column(name))
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
    let mut rows = BTreeMap::new();
    let mut children: ModifierChildren = BTreeMap::new();
    for record in table.records() {
        let values = columns
            .iter()
            .map(|column| {
                record
                    .get(*column)
                    .ok_or("ModifierTree missing cell")?
                    .parse::<i64>()
                    .map_err(|error| format!("ModifierTree: {error}"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let [id, parent, op, amount, kind, asset, second, third]: [i64; 8] =
            values.try_into().expect("eight fields requested");
        rows.insert(id as u32, [op, amount, kind, asset, second, third]);
        children.entry(parent as u32).or_default().push(id as u32);
    }
    Ok((rows, children))
}

fn build_leaf(id: u32, fields: [i64; 6]) -> Result<Predicate, String> {
    let [op, _, kind, asset, second, third] = fields;
    if kind == 0 {
        return Ok(Predicate::False);
    }
    if second != 0 || third != 0 {
        return Err(format!(
            "ModifierTree {id}: unsupported secondary/tertiary asset"
        ));
    }
    let asset =
        u32::try_from(asset).map_err(|error| format!("ModifierTree {id} asset: {error}"))?;
    let leaf = match kind {
        8 => Predicate::Aura(asset),
        104 => Predicate::KnownSpell(asset),
        _ => return Err(format!("ModifierTree {id}: unsupported type {kind}")),
    };
    Ok(if op == 3 {
        Predicate::Not(Box::new(leaf))
    } else {
        leaf
    })
}

fn build_predicate(
    id: u32,
    rows: &ModifierRows,
    children: &ModifierChildren,
) -> Result<Predicate, String> {
    let fields = *rows
        .get(&id)
        .ok_or_else(|| format!("ModifierTree {id}: absent authored row"))?;
    let [op, amount, _, _, _, _] = fields;
    match op {
        2 | 3 => build_leaf(id, fields),
        4 | 8 => {
            let predicates = children
                .get(&id)
                .into_iter()
                .flatten()
                .map(|child| build_predicate(*child, rows, children))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(if op == 4 {
                Predicate::All(predicates)
            } else {
                Predicate::Some(amount.max(1) as u32, predicates)
            })
        }
        _ => Err(format!("ModifierTree {id}: unsupported operator {op}")),
    }
}
