use std::{collections::BTreeMap, time::Duration};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use starter_domain::{Item, ItemId, ItemName};

use crate::{
    AppError, AppEvent, Clock, Command, ItemRepository, Output,
    jobs::{CancelToken, JobId, JobInfo, JobRequest, JobRunner, JobState, PreparedJob, ReportTask},
};

/// How many finished jobs are kept for display before the oldest are dropped.
const FINISHED_JOBS_KEPT: usize = 50;

/// Storage location and counters, returned by [`Command::Info`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct AppInfo {
    pub storage: String,
    pub item_count: u64,
    pub running_jobs: u64,
}

struct JobEntry {
    info: JobInfo,
    cancel: CancelToken,
}

/// The application service. Owns the ports and is the single entry point for every frontend.
///
/// `App` has exactly one owner (the UI thread, the CLI's main thread, a test). It is not shared
/// between threads; long-running work leaves it as a job and comes back through [`App::poll`].
pub struct App {
    repo: Box<dyn ItemRepository>,
    jobs: Box<dyn JobRunner>,
    clock: Box<dyn Clock>,
    job_table: BTreeMap<JobId, JobEntry>,
    next_job_id: u64,
    events: Vec<AppEvent>,
}

impl App {
    pub fn new(repo: Box<dyn ItemRepository>, jobs: Box<dyn JobRunner>, clock: Box<dyn Clock>) -> Self {
        Self {
            repo,
            jobs,
            clock,
            job_table: BTreeMap::new(),
            next_job_id: 1,
            events: Vec::new(),
        }
    }

    /// Executes one command. This is the only way frontends change or read application state.
    pub fn execute(&mut self, command: Command) -> Result<Output, AppError> {
        log::debug!("execute {}", command.name());
        match command {
            Command::CreateItem { name } => self.create_item(&name),
            Command::RenameItem { id, name } => self.rename_item(id, &name),
            Command::DeleteItem { id } => self.delete_item(id),
            Command::ListItems => Ok(Output::Items(self.repo.list()?)),
            Command::GetItem { id } => Ok(Output::Item(self.find_item(id)?)),
            Command::Info => Ok(Output::Info(self.info()?)),
            Command::StartJob { job } => self.start_job(job),
            Command::CancelJob { id } => self.cancel_job(id),
            Command::ListJobs => Ok(Output::Jobs(self.job_table.values().map(|e| e.info.clone()).collect())),
        }
    }

    /// Collects job updates from the runner. Call it regularly (every UI frame, or in a CLI wait loop).
    pub fn poll(&mut self) {
        for update in self.jobs.poll() {
            let Some(entry) = self.job_table.get_mut(&update.id) else {
                continue;
            };
            if entry.info.state.is_finished() {
                continue;
            }
            entry.info.state = update.state;
            self.events.push(AppEvent::JobChanged {
                job: entry.info.clone(),
            });
        }
        self.prune_finished_jobs();
    }

    /// Returns and clears all events since the last call.
    pub fn drain_events(&mut self) -> Vec<AppEvent> {
        std::mem::take(&mut self.events)
    }

    /// `true` while at least one job is queued or running. UIs use it to keep repainting.
    pub fn has_active_jobs(&self) -> bool {
        self.job_table.values().any(|e| !e.info.state.is_finished())
    }

    fn create_item(&mut self, raw_name: &str) -> Result<Output, AppError> {
        let name = ItemName::parse(raw_name)?;
        let item = Item::new(ItemId::generate(), name, self.clock.now());
        self.repo.insert(&item)?;
        self.events.push(AppEvent::ItemCreated { id: item.id });
        Ok(Output::Item(item))
    }

    fn rename_item(&mut self, id: ItemId, raw_name: &str) -> Result<Output, AppError> {
        let name = ItemName::parse(raw_name)?;
        let mut item = self.find_item(id)?;
        item.rename(name, self.clock.now());
        self.repo.update(&item)?;
        self.events.push(AppEvent::ItemUpdated { id });
        Ok(Output::Item(item))
    }

    fn delete_item(&mut self, id: ItemId) -> Result<Output, AppError> {
        if !self.repo.delete(id)? {
            return Err(AppError::NotFound {
                what: "item",
                id: id.to_string(),
            });
        }
        self.events.push(AppEvent::ItemDeleted { id });
        Ok(Output::Deleted { id })
    }

    fn find_item(&self, id: ItemId) -> Result<Item, AppError> {
        self.repo.get(id)?.ok_or_else(|| AppError::NotFound {
            what: "item",
            id: id.to_string(),
        })
    }

    fn info(&self) -> Result<AppInfo, AppError> {
        let running = self.job_table.values().filter(|e| !e.info.state.is_finished()).count();
        Ok(AppInfo {
            storage: self.repo.describe(),
            item_count: u64::try_from(self.repo.list()?.len()).unwrap_or(u64::MAX),
            running_jobs: u64::try_from(running).unwrap_or(u64::MAX),
        })
    }

    fn start_job(&mut self, request: JobRequest) -> Result<Output, AppError> {
        let id = JobId::new(self.next_job_id);
        self.next_job_id += 1;
        let cancel = CancelToken::default();
        let (task, step_delay) = match &request {
            JobRequest::GenerateReport { step_delay_ms } => {
                // Snapshot the input on the owner thread; the task never touches the repository.
                let names = self
                    .repo
                    .list()?
                    .into_iter()
                    .map(|item| item.name.to_string())
                    .collect();
                (
                    Box::new(ReportTask::new(names)),
                    Duration::from_millis(u64::from(*step_delay_ms)),
                )
            }
        };
        let info = JobInfo {
            id,
            label: request.label().to_owned(),
            state: JobState::Queued,
        };
        self.events.push(AppEvent::JobChanged { job: info.clone() });
        self.job_table.insert(
            id,
            JobEntry {
                info,
                cancel: cancel.clone(),
            },
        );
        self.jobs.submit(PreparedJob {
            id,
            task,
            cancel,
            step_delay,
        });
        Ok(Output::JobStarted { id })
    }

    fn cancel_job(&mut self, id: JobId) -> Result<Output, AppError> {
        let entry = self.job_table.get(&id).ok_or_else(|| AppError::NotFound {
            what: "job",
            id: id.to_string(),
        })?;
        entry.cancel.cancel();
        Ok(Output::CancelRequested { id })
    }

    fn prune_finished_jobs(&mut self) {
        let finished: Vec<JobId> = self
            .job_table
            .iter()
            .filter(|(_, e)| e.info.state.is_finished())
            .map(|(id, _)| *id)
            .collect();
        let excess = finished.len().saturating_sub(FINISHED_JOBS_KEPT);
        for id in finished.into_iter().take(excess) {
            self.job_table.remove(&id);
        }
    }
}

#[cfg(test)]
mod tests {
    use starter_domain::{DomainError, Timestamp};

    use super::*;
    use crate::{
        InMemoryItemRepository, ManualClock,
        jobs::{InlineJobRunner, JobOutput},
    };

    fn app() -> (App, ManualClock) {
        let clock = ManualClock::new(Timestamp::from_millis(1_000));
        let app = App::new(
            Box::new(InMemoryItemRepository::new()),
            Box::new(InlineJobRunner::new(Duration::from_secs(1))),
            Box::new(clock.clone()),
        );
        (app, clock)
    }

    fn create(app: &mut App, name: &str) -> Item {
        match app.execute(Command::CreateItem { name: name.into() }).unwrap() {
            Output::Item(item) => item,
            other => panic!("unexpected output {other:?}"),
        }
    }

    #[test]
    fn create_and_list() {
        let (mut app, clock) = app();
        create(&mut app, "first");
        clock.advance_millis(10);
        create(&mut app, "second");
        let Output::Items(items) = app.execute(Command::ListItems).unwrap() else {
            panic!("expected items")
        };
        let names: Vec<&str> = items.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(names, ["first", "second"]);
    }

    #[test]
    fn create_rejects_invalid_name() {
        let (mut app, _) = app();
        let err = app.execute(Command::CreateItem { name: "  ".into() }).unwrap_err();
        assert!(matches!(err, AppError::Validation(DomainError::EmptyName)));
        assert_eq!(err.code(), "validation");
    }

    #[test]
    fn rename_updates_timestamp_and_emits_event() {
        let (mut app, clock) = app();
        let item = create(&mut app, "old");
        app.drain_events();
        clock.advance_millis(500);
        let Output::Item(renamed) = app
            .execute(Command::RenameItem {
                id: item.id,
                name: "new".into(),
            })
            .unwrap()
        else {
            panic!("expected item")
        };
        assert_eq!(renamed.name.as_str(), "new");
        assert_eq!(renamed.updated_at, Timestamp::from_millis(1_500));
        assert_eq!(app.drain_events(), vec![AppEvent::ItemUpdated { id: item.id }]);
    }

    #[test]
    fn delete_unknown_is_not_found() {
        let (mut app, _) = app();
        let err = app.execute(Command::DeleteItem { id: ItemId::generate() }).unwrap_err();
        assert_eq!(err.code(), "not_found");
    }

    #[test]
    fn report_job_runs_to_completion() {
        let (mut app, _) = app();
        create(&mut app, "abc");
        create(&mut app, "de");
        let Output::JobStarted { id } = app
            .execute(Command::StartJob {
                job: JobRequest::GenerateReport { step_delay_ms: 0 },
            })
            .unwrap()
        else {
            panic!("expected job id")
        };
        app.poll();
        assert!(!app.has_active_jobs());
        let Output::Jobs(jobs) = app.execute(Command::ListJobs).unwrap() else {
            panic!("expected jobs")
        };
        let job = jobs.iter().find(|j| j.id == id).unwrap();
        assert_eq!(
            job.state,
            JobState::Succeeded {
                output: JobOutput::Report {
                    item_count: 2,
                    total_characters: 5,
                    longest_name: Some("abc".into())
                }
            }
        );
    }

    #[test]
    fn cancel_stops_a_paced_job() {
        let (mut app, _) = app();
        create(&mut app, "a");
        create(&mut app, "b");
        let Output::JobStarted { id } = app
            .execute(Command::StartJob {
                job: JobRequest::GenerateReport { step_delay_ms: 60_000 },
            })
            .unwrap()
        else {
            panic!("expected job id")
        };
        app.poll(); // first step runs, the next one is a minute away
        assert!(app.has_active_jobs());
        app.execute(Command::CancelJob { id }).unwrap();
        app.poll();
        assert!(!app.has_active_jobs());
        let Output::Jobs(jobs) = app.execute(Command::ListJobs).unwrap() else {
            panic!("expected jobs")
        };
        assert_eq!(jobs[0].state, JobState::Cancelled);
    }

    #[test]
    fn info_reports_counts() {
        let (mut app, _) = app();
        create(&mut app, "a");
        let Output::Info(info) = app.execute(Command::Info).unwrap() else {
            panic!("expected info")
        };
        assert_eq!(info.item_count, 1);
        assert_eq!(info.running_jobs, 0);
    }
}
