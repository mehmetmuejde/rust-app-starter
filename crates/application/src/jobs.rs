//! Background job model.
//!
//! A job is prepared on the owner thread ([`crate::App`]) as a self-contained [`JobTask`] that owns
//! all its input data. A [`JobRunner`] decides *where* the task runs: a native thread pool, inline
//! slices on the browser's main thread, or (later) an external worker. Runners report back through
//! [`JobRunner::poll`]; the `App` applies results on the owner thread.
//!
//! Tasks work in small [`JobTask::step`]s. That gives progress reporting and cancellation for free and
//! lets single-threaded runners (browser) interleave work with UI frames.

use std::{
    collections::VecDeque,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use web_time::Instant;

/// Identifier of a job, unique within one `App` instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct JobId(u64);

impl JobId {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for JobId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// The jobs a frontend can start.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum JobRequest {
    /// Summarise all items. `step_delay_ms` slows every step down so progress is visible in demos.
    GenerateReport {
        #[serde(default)]
        step_delay_ms: u32,
    },
}

impl JobRequest {
    pub fn label(&self) -> &'static str {
        match self {
            Self::GenerateReport { .. } => "Generate report",
        }
    }
}

/// Result of a successful job.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum JobOutput {
    Report {
        item_count: u32,
        total_characters: u64,
        longest_name: Option<String>,
    },
}

/// Lifecycle of a job.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum JobState {
    Queued,
    Running { done: u32, total: u32 },
    Succeeded { output: JobOutput },
    Failed { error: String },
    Cancelled,
}

impl JobState {
    pub fn is_finished(&self) -> bool {
        matches!(self, Self::Succeeded { .. } | Self::Failed { .. } | Self::Cancelled)
    }
}

/// A job as frontends see it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct JobInfo {
    pub id: JobId,
    pub label: String,
    pub state: JobState,
}

/// Cooperative cancellation flag shared between the owner and the running task.
#[derive(Debug, Clone, Default)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// Outcome of one [`JobTask::step`].
#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    Progress { done: u32, total: u32 },
    Finished(Result<JobOutput, String>),
}

/// Self-contained unit of background work. Must not touch the repository; it owns its input.
pub trait JobTask: Send {
    /// Performs one bounded slice of work.
    fn step(&mut self) -> Step;
}

/// A task plus everything a runner needs to execute it.
pub struct PreparedJob {
    pub id: JobId,
    pub task: Box<dyn JobTask>,
    pub cancel: CancelToken,
    /// Minimum pause between steps (demo pacing). Zero means "as fast as possible".
    pub step_delay: Duration,
}

impl PreparedJob {
    /// Runs one step, honouring cancellation, and returns the resulting state.
    pub fn advance(&mut self) -> JobState {
        if self.cancel.is_cancelled() {
            return JobState::Cancelled;
        }
        match self.task.step() {
            Step::Progress { done, total } => JobState::Running { done, total },
            Step::Finished(Ok(output)) => JobState::Succeeded { output },
            Step::Finished(Err(error)) => JobState::Failed { error },
        }
    }
}

/// State change reported by a runner.
#[derive(Debug, Clone, PartialEq)]
pub struct JobUpdate {
    pub id: JobId,
    pub state: JobState,
}

/// Executes prepared jobs somewhere and reports progress.
pub trait JobRunner {
    fn submit(&mut self, job: PreparedJob);
    /// Returns all updates since the last call. Called by `App::poll` on the owner thread.
    fn poll(&mut self) -> Vec<JobUpdate>;
}

/// Runs jobs on the calling thread, a little on every [`JobRunner::poll`].
///
/// The runner for the browser (no threads) and for deterministic tests. Each poll spends at most
/// `budget` of work on the front job, so a UI frame never blocks for long.
pub struct InlineJobRunner {
    queue: VecDeque<(PreparedJob, Option<Instant>)>,
    budget: Duration,
}

impl InlineJobRunner {
    pub fn new(budget: Duration) -> Self {
        Self {
            queue: VecDeque::new(),
            budget,
        }
    }
}

impl Default for InlineJobRunner {
    fn default() -> Self {
        Self::new(Duration::from_millis(8))
    }
}

impl JobRunner for InlineJobRunner {
    fn submit(&mut self, job: PreparedJob) {
        self.queue.push_back((job, None));
    }

    fn poll(&mut self) -> Vec<JobUpdate> {
        let mut updates = Vec::new();
        let started = Instant::now();
        while let Some((job, next_due)) = self.queue.front_mut() {
            if job.cancel.is_cancelled() {
                updates.push(JobUpdate {
                    id: job.id,
                    state: JobState::Cancelled,
                });
                self.queue.pop_front();
                continue;
            }
            let now = Instant::now();
            if next_due.is_some_and(|due| now < due) {
                break;
            }
            let state = job.advance();
            let finished = state.is_finished();
            updates.push(JobUpdate { id: job.id, state });
            if finished {
                self.queue.pop_front();
            } else if !job.step_delay.is_zero() {
                *next_due = Some(now + job.step_delay);
                break;
            }
            if started.elapsed() >= self.budget {
                break;
            }
        }
        updates
    }
}

/// Task behind [`JobRequest::GenerateReport`]: one item per step.
pub(crate) struct ReportTask {
    names: Vec<String>,
    next: usize,
    total_characters: u64,
    longest: Option<String>,
}

impl ReportTask {
    pub(crate) fn new(names: Vec<String>) -> Self {
        Self {
            names,
            next: 0,
            total_characters: 0,
            longest: None,
        }
    }
}

impl JobTask for ReportTask {
    fn step(&mut self) -> Step {
        let total = u32::try_from(self.names.len()).unwrap_or(u32::MAX);
        let Some(name) = self.names.get(self.next) else {
            return Step::Finished(Ok(JobOutput::Report {
                item_count: total,
                total_characters: self.total_characters,
                longest_name: self.longest.take(),
            }));
        };
        let chars = name.chars().count();
        self.total_characters += u64::try_from(chars).unwrap_or(u64::MAX);
        if self.longest.as_ref().is_none_or(|l| chars > l.chars().count()) {
            self.longest = Some(name.clone());
        }
        self.next += 1;
        Step::Progress {
            done: u32::try_from(self.next).unwrap_or(u32::MAX),
            total,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prepared(names: &[&str]) -> PreparedJob {
        PreparedJob {
            id: JobId::new(1),
            task: Box::new(ReportTask::new(names.iter().map(|s| (*s).to_owned()).collect())),
            cancel: CancelToken::default(),
            step_delay: Duration::ZERO,
        }
    }

    #[test]
    fn report_counts_items_and_characters() {
        let mut job = prepared(&["ab", "abcd", "c"]);
        let mut last = JobState::Queued;
        while !last.is_finished() {
            last = job.advance();
        }
        assert_eq!(
            last,
            JobState::Succeeded {
                output: JobOutput::Report {
                    item_count: 3,
                    total_characters: 7,
                    longest_name: Some("abcd".into())
                }
            }
        );
    }

    #[test]
    fn cancelled_job_stops() {
        let mut job = prepared(&["a", "b"]);
        job.cancel.cancel();
        assert_eq!(job.advance(), JobState::Cancelled);
    }

    #[test]
    fn inline_runner_finishes_within_polls() {
        let mut runner = InlineJobRunner::new(Duration::from_secs(1));
        runner.submit(prepared(&["a", "b"]));
        let updates = runner.poll();
        assert!(updates.last().is_some_and(|u| u.state.is_finished()));
        assert!(runner.poll().is_empty());
    }
}
