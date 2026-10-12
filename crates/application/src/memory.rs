use std::collections::BTreeMap;

use starter_domain::{Item, ItemId};

use crate::{ItemRepository, StorageError};

/// Repository held entirely in memory.
///
/// Used by tests, and as the working state of the browser adapter, which persists snapshots of it.
#[derive(Debug, Clone, Default)]
pub struct InMemoryItemRepository {
    items: BTreeMap<ItemId, Item>,
}

impl InMemoryItemRepository {
    pub fn new() -> Self {
        Self::default()
    }

    /// Builds the repository from previously stored items (e.g. a loaded snapshot).
    pub fn from_items(items: impl IntoIterator<Item = Item>) -> Self {
        Self {
            items: items.into_iter().map(|item| (item.id, item)).collect(),
        }
    }
}

impl ItemRepository for InMemoryItemRepository {
    fn insert(&mut self, item: &Item) -> Result<(), StorageError> {
        if self.items.contains_key(&item.id) {
            return Err(StorageError::Duplicate(item.id.to_string()));
        }
        self.items.insert(item.id, item.clone());
        Ok(())
    }

    fn update(&mut self, item: &Item) -> Result<(), StorageError> {
        match self.items.get_mut(&item.id) {
            Some(slot) => {
                *slot = item.clone();
                Ok(())
            }
            None => Err(StorageError::Missing(item.id.to_string())),
        }
    }

    fn delete(&mut self, id: ItemId) -> Result<bool, StorageError> {
        Ok(self.items.remove(&id).is_some())
    }

    fn get(&self, id: ItemId) -> Result<Option<Item>, StorageError> {
        Ok(self.items.get(&id).cloned())
    }

    fn list(&self) -> Result<Vec<Item>, StorageError> {
        let mut items: Vec<Item> = self.items.values().cloned().collect();
        items.sort_by(|a, b| a.created_at.cmp(&b.created_at).then(a.id.cmp(&b.id)));
        Ok(items)
    }

    fn describe(&self) -> String {
        "in memory (not persisted)".to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn satisfies_repository_contract() {
        crate::contract::run_all(&mut || Box::new(InMemoryItemRepository::new()));
    }
}
