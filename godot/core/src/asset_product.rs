//! Authored metadata ownership, separate from the actual extracted asset build.
use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ValueRef};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AssetProduct {
    #[serde(rename = "wow")]
    Retail,
    #[serde(rename = "wow_classic_beta")]
    Forever,
}

impl AssetProduct {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Retail => "wow",
            Self::Forever => "wow_classic_beta",
        }
    }
}

impl FromSql for AssetProduct {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match value.as_str()? {
            "wow" => Ok(Self::Retail),
            "wow_classic_beta" => Ok(Self::Forever),
            _ => Err(FromSqlError::InvalidType),
        }
    }
}
