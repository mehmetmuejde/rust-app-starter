//! Repository contract tests: one behavioural test suite that every [`ItemRepository`] adapter must
//! pass. Adapter crates enable the `contract-tests` feature as a dev-dependency and call [`run_all`].
#![allow(clippy::panic, clippy::unwrap_used, clippy::expect_used)]

use starter_domain::{Item, ItemId, ItemName, Timestamp};

use crate::{ItemRepository, StorageError};

/// Runs every contract check. `make` must return a fresh, empty repository on each call.
pub fn run_all(make: &mut dyn FnMut() -> Box<dyn ItemRepository>) {
    insert_then_get(make());
    insert_duplicate_fails(make());
    update_replaces(make());
    update_missing_fails(make());
    delete_reports_existence(make());
    list_is_ordered_by_creation(make());
    get_unknown_is_none(make());
}

/// Builds an item with a fixed creation time.
pub fn item(name: &str, created_millis: i64) -> Item {
    Item::new(
        ItemId::generate(),
        ItemName::parse(name).unwrap(),
        Timestamp::from_millis(created_millis),
    )
}

fn insert_then_get(mut repo: Box<dyn ItemRepository>) {
    let a = item("alpha", 1);
    repo.insert(&a).unwrap();
    assert_eq!(repo.get(a.id).unwrap(), Some(a), "inserted item must be readable");
}

fn insert_duplicate_fails(mut repo: Box<dyn ItemRepository>) {
    let a = item("alpha", 1);
    repo.insert(&a).unwrap();
    let err = repo.insert(&a).unwrap_err();
    assert!(
        matches!(err, StorageError::Duplicate(_)),
        "duplicate insert must fail, got {err:?}"
    );
}

fn update_replaces(mut repo: Box<dyn ItemRepository>) {
    let mut a = item("alpha", 1);
    repo.insert(&a).unwrap();
    a.rename(ItemName::parse("beta").unwrap(), Timestamp::from_millis(9));
    repo.update(&a).unwrap();
    assert_eq!(repo.get(a.id).unwrap(), Some(a), "update must replace the stored item");
}

fn update_missing_fails(mut repo: Box<dyn ItemRepository>) {
    let err = repo.update(&item("ghost", 1)).unwrap_err();
    assert!(
        matches!(err, StorageError::Missing(_)),
        "update of unknown id must fail, got {err:?}"
    );
}

fn delete_reports_existence(mut repo: Box<dyn ItemRepository>) {
    let a = item("alpha", 1);
    repo.insert(&a).unwrap();
    assert!(repo.delete(a.id).unwrap(), "deleting an existing item returns true");
    assert!(!repo.delete(a.id).unwrap(), "deleting a missing item returns false");
    assert_eq!(repo.get(a.id).unwrap(), None);
}

fn list_is_ordered_by_creation(mut repo: Box<dyn ItemRepository>) {
    let late = item("late", 30);
    let early = item("early", 10);
    let middle = item("middle", 20);
    for it in [&late, &early, &middle] {
        repo.insert(it).unwrap();
    }
    let names: Vec<String> = repo.list().unwrap().into_iter().map(|i| i.name.to_string()).collect();
    assert_eq!(names, ["early", "middle", "late"]);
}

fn get_unknown_is_none(repo: Box<dyn ItemRepository>) {
    assert_eq!(repo.get(ItemId::generate()).unwrap(), None);
}
