use std::{fmt, str::FromStr};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{DomainError, Timestamp};

/// Identifier of an [`Item`]: a UUIDv7, so ids sort by creation time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct ItemId(Uuid);

impl ItemId {
    /// Creates a new, random, time-ordered id.
    pub fn generate() -> Self {
        Self(Uuid::now_v7())
    }

    pub const fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub const fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl fmt::Display for ItemId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for ItemId {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(s.trim())
            .map(Self)
            .map_err(|_| DomainError::InvalidId(s.to_owned()))
    }
}

/// A validated item name: trimmed, not empty, at most [`ItemName::MAX_CHARS`] characters.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "String", into = "String")]
pub struct ItemName(String);

impl ItemName {
    pub const MAX_CHARS: usize = 120;

    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(DomainError::EmptyName);
        }
        if trimmed.chars().count() > Self::MAX_CHARS {
            return Err(DomainError::NameTooLong { max: Self::MAX_CHARS });
        }
        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ItemName {
    type Error = DomainError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}

impl From<ItemName> for String {
    fn from(value: ItemName) -> Self {
        value.0
    }
}

impl fmt::Display for ItemName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// The reference entity. Fields are public for reading; changes go through methods that keep the
/// invariants (e.g. `updated_at` moves with every rename).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Item {
    pub id: ItemId,
    pub name: ItemName,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

impl Item {
    pub fn new(id: ItemId, name: ItemName, now: Timestamp) -> Self {
        Self {
            id,
            name,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn rename(&mut self, name: ItemName, now: Timestamp) {
        self.name = name;
        self.updated_at = now;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_is_trimmed() {
        assert_eq!(ItemName::parse("  Project  ").unwrap().as_str(), "Project");
    }

    #[test]
    fn empty_name_is_rejected() {
        assert_eq!(ItemName::parse("   "), Err(DomainError::EmptyName));
    }

    #[test]
    fn long_name_is_rejected() {
        let raw = "x".repeat(ItemName::MAX_CHARS + 1);
        assert_eq!(
            ItemName::parse(&raw),
            Err(DomainError::NameTooLong {
                max: ItemName::MAX_CHARS
            })
        );
    }

    #[test]
    fn name_limit_counts_characters_not_bytes() {
        let raw = "ü".repeat(ItemName::MAX_CHARS);
        assert!(ItemName::parse(&raw).is_ok());
    }

    #[test]
    fn invalid_name_fails_deserialisation() {
        let result: Result<ItemName, _> = serde_json::from_str("\"  \"");
        assert!(result.is_err());
    }

    #[test]
    fn rename_moves_updated_at() {
        let mut item = Item::new(
            ItemId::generate(),
            ItemName::parse("a").unwrap(),
            Timestamp::from_millis(1),
        );
        item.rename(ItemName::parse("b").unwrap(), Timestamp::from_millis(5));
        assert_eq!(item.created_at, Timestamp::from_millis(1));
        assert_eq!(item.updated_at, Timestamp::from_millis(5));
    }

    #[test]
    fn id_roundtrips_through_string() {
        let id = ItemId::generate();
        assert_eq!(id.to_string().parse::<ItemId>().unwrap(), id);
        assert!("nope".parse::<ItemId>().is_err());
    }
}
