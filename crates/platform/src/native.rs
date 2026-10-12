//! Native host services: worker threads, OS data directories, logging to stderr.

use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex, PoisonError,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread::JoinHandle,
    time::Duration,
};

use starter_application::jobs::{JobRunner, JobUpdate, PreparedJob};

/// Longest uninterrupted sleep between steps, so cancellation stays responsive.
const SLEEP_SLICE: Duration = Duration::from_millis(50);

/// Callback invoked from worker threads after every update (e.g. `egui::Context::request_repaint`).
pub type Notifier = Arc<dyn Fn() + Send + Sync>;

/// Runs jobs on a small, fixed pool of OS threads.
///
/// Jobs are CPU or blocking work; inside a step a job may use `rayon` for data parallelism.
/// Dropping the runner cancels running jobs and joins the workers.
pub struct ThreadJobRunner {
    sender: Option<mpsc::Sender<PreparedJob>>,
    updates: mpsc::Receiver<JobUpdate>,
    workers: Vec<JoinHandle<()>>,
    shutdown: Arc<AtomicBool>,
}

impl ThreadJobRunner {
    /// Starts `threads` workers (at least one).
    pub fn new(threads: usize, notifier: Option<Notifier>) -> Self {
        let (sender, jobs) = mpsc::channel::<PreparedJob>();
        let (update_tx, updates) = mpsc::channel();
        let jobs = Arc::new(Mutex::new(jobs));
        let shutdown = Arc::new(AtomicBool::new(false));
        let workers = (0..threads.max(1))
            .filter_map(|index| {
                let jobs = Arc::clone(&jobs);
                let updates = update_tx.clone();
                let shutdown = Arc::clone(&shutdown);
                let notifier = notifier.clone();
                std::thread::Builder::new()
                    .name(format!("job-worker-{index}"))
                    .spawn(move || worker_loop(&jobs, &updates, &shutdown, notifier.as_deref()))
                    .map_err(|err| log::error!("could not start job worker {index}: {err}"))
                    .ok()
            })
            .collect();
        Self {
            sender: Some(sender),
            updates,
            workers,
            shutdown,
        }
    }

    /// Worker count from the available cores, clamped to 1..=4.
    pub fn default_threads() -> usize {
        std::thread::available_parallelism()
            .map_or(1, |n| n.get().saturating_sub(1))
            .clamp(1, 4)
    }
}

fn worker_loop(
    jobs: &Mutex<mpsc::Receiver<PreparedJob>>,
    updates: &mpsc::Sender<JobUpdate>,
    shutdown: &AtomicBool,
    notifier: Option<&(dyn Fn() + Send + Sync)>,
) {
    loop {
        let next = jobs.lock().unwrap_or_else(PoisonError::into_inner).recv();
        let Ok(mut job) = next else {
            return; // runner dropped
        };
        loop {
            if shutdown.load(Ordering::SeqCst) {
                job.cancel.cancel();
            }
            let state = job.advance();
            let finished = state.is_finished();
            if updates.send(JobUpdate { id: job.id, state }).is_err() {
                return;
            }
            if let Some(notify) = notifier {
                notify();
            }
            if finished {
                break;
            }
            sleep_unless_cancelled(job.step_delay, &job);
        }
    }
}

fn sleep_unless_cancelled(delay: Duration, job: &PreparedJob) {
    let mut remaining = delay;
    while !remaining.is_zero() && !job.cancel.is_cancelled() {
        let slice = remaining.min(SLEEP_SLICE);
        std::thread::sleep(slice);
        remaining = remaining.saturating_sub(slice);
    }
}

impl JobRunner for ThreadJobRunner {
    fn submit(&mut self, job: PreparedJob) {
        let id = job.id;
        let sent = self.sender.as_ref().is_some_and(|sender| sender.send(job).is_ok());
        if !sent {
            log::error!("job {id} rejected: runner is shut down");
        }
    }

    fn poll(&mut self) -> Vec<JobUpdate> {
        self.updates.try_iter().collect()
    }
}

impl Drop for ThreadJobRunner {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
        self.sender = None; // workers leave `recv` once the queue is empty
        for worker in self.workers.drain(..) {
            if worker.join().is_err() {
                log::error!("a job worker panicked");
            }
        }
    }
}

/// Per-user data directory, e.g. `~/Library/Application Support/dev.rust-app-starter.rust-app` on macOS,
/// `~/.local/share/rust-app` on Linux, `%APPDATA%\rust-app-starter\rust-app\data` on Windows.
pub fn default_data_dir(app_name: &str) -> Option<PathBuf> {
    directories::ProjectDirs::from("dev", "rust-app-starter", app_name).map(|dirs| dirs.data_dir().to_path_buf())
}

/// Logs to stderr. Level from `RUST_LOG` (default `info`). Safe to call more than once.
pub fn init_logging() {
    let env = env_logger::Env::default().default_filter_or("info");
    if env_logger::Builder::from_env(env).try_init().is_err() {
        log::debug!("logger already initialised");
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use starter_application::{
        App, Command, InMemoryItemRepository, ManualClock, Output,
        jobs::{JobOutput, JobRequest, JobState},
    };
    use starter_domain::Timestamp;

    use super::*;

    fn app() -> App {
        App::new(
            Box::new(InMemoryItemRepository::new()),
            Box::new(ThreadJobRunner::new(2, None)),
            Box::new(ManualClock::new(Timestamp::from_millis(0))),
        )
    }

    fn wait_until_idle(app: &mut App) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while app.has_active_jobs() {
            assert!(Instant::now() < deadline, "job did not finish in time");
            std::thread::sleep(Duration::from_millis(5));
            app.poll();
        }
    }

    #[test]
    fn report_job_finishes_on_worker_thread() {
        let mut app = app();
        app.execute(Command::CreateItem { name: "abc".into() }).unwrap();
        app.execute(Command::StartJob {
            job: JobRequest::GenerateReport { step_delay_ms: 0 },
        })
        .unwrap();
        wait_until_idle(&mut app);
        let Output::Jobs(jobs) = app.execute(Command::ListJobs).unwrap() else {
            panic!("expected jobs")
        };
        assert_eq!(
            jobs[0].state,
            JobState::Succeeded {
                output: JobOutput::Report {
                    item_count: 1,
                    total_characters: 3,
                    longest_name: Some("abc".into())
                }
            }
        );
    }

    #[test]
    fn cancellation_interrupts_paced_job() {
        let mut app = app();
        for name in ["a", "b", "c"] {
            app.execute(Command::CreateItem { name: name.into() }).unwrap();
        }
        let Output::JobStarted { id } = app
            .execute(Command::StartJob {
                job: JobRequest::GenerateReport { step_delay_ms: 10_000 },
            })
            .unwrap()
        else {
            panic!("expected job id")
        };
        app.execute(Command::CancelJob { id }).unwrap();
        wait_until_idle(&mut app);
        let Output::Jobs(jobs) = app.execute(Command::ListJobs).unwrap() else {
            panic!("expected jobs")
        };
        assert_eq!(jobs[0].state, JobState::Cancelled);
    }

    #[test]
    fn drop_joins_workers_quickly() {
        let mut app = app();
        app.execute(Command::CreateItem { name: "a".into() }).unwrap();
        app.execute(Command::StartJob {
            job: JobRequest::GenerateReport { step_delay_ms: 10_000 },
        })
        .unwrap();
        let started = Instant::now();
        drop(app);
        assert!(started.elapsed() < Duration::from_secs(2));
    }
}
