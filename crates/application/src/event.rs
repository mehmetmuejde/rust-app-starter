use serde::{Deserialize, Serialize};
use starter_domain::ItemId;

use crate::jobs::JobInfo;

/// Something changed. Frontends pull events with [`crate::App::drain_events`] and refresh what they
/// show; the event log is also the natural hook for audit trails or automation later.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum AppEvent {
    ItemCreated { id: ItemId },
    ItemUpdated { id: ItemId },
    ItemDeleted { id: ItemId },
    JobChanged { job: JobInfo },
}
