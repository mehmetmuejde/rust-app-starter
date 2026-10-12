use std::time::Duration;

use egui::{CentralPanel, Panel, RichText};
use starter_application::{
    App, AppError, AppInfo, Command, Output,
    domain::{Item, ItemId},
    jobs::{JobInfo, JobRequest},
};

/// Delay between report steps when started from the UI, so progress is visible.
const DEMO_STEP_DELAY_MS: u32 = 150;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum View {
    Items,
    Jobs,
    About,
}

pub(crate) struct Status {
    pub(crate) text: String,
    pub(crate) is_error: bool,
}

/// UI state of the reference application. The data shown is a cache of command outputs, refreshed
/// whenever the core reports an event.
pub struct Shell {
    platform: String,
    pub(crate) view: View,
    pub(crate) new_name: String,
    pub(crate) editing: Option<(ItemId, String)>,
    pub(crate) confirm_delete: Option<ItemId>,
    pub(crate) items: Vec<Item>,
    pub(crate) jobs: Vec<JobInfo>,
    pub(crate) info: Option<AppInfo>,
    pub(crate) status: Status,
    needs_refresh: bool,
    /// Commands requested while drawing; executed after the frame's widgets are laid out.
    pub(crate) pending: Vec<Command>,
}

impl Shell {
    /// `platform` is shown in the About view, e.g. "Desktop (macos)" or "Browser".
    pub fn new(platform: impl Into<String>) -> Self {
        Self {
            platform: platform.into(),
            view: View::Items,
            new_name: String::new(),
            editing: None,
            confirm_delete: None,
            items: Vec::new(),
            jobs: Vec::new(),
            info: None,
            status: Status {
                text: "Ready".to_owned(),
                is_error: false,
            },
            needs_refresh: true,
            pending: Vec::new(),
        }
    }

    /// Non-drawing work, once per frame: collect job progress, react to events, refresh caches.
    /// Call it from `eframe::App::logic` (or before [`Shell::ui`] in other hosts).
    pub fn logic(&mut self, ctx: &egui::Context, app: &mut App) {
        app.poll();
        if !app.drain_events().is_empty() {
            self.needs_refresh = true;
        }
        if self.needs_refresh {
            self.refresh(app);
        }
        if app.has_active_jobs() {
            // Keep frames coming so the inline runner (browser) advances and progress bars move.
            ctx.request_repaint_after(Duration::from_millis(16));
        }
    }

    /// Draws the application into `ui` (the full window) and runs the commands the user triggered.
    pub fn ui(&mut self, ui: &mut egui::Ui, app: &mut App) {
        Panel::top("top_bar").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Rust App Starter");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    egui::widgets::global_theme_preference_switch(ui);
                });
            });
        });
        Panel::bottom("status_bar").show(ui, |ui| self.status_bar(ui));
        Panel::left("navigation")
            .resizable(false)
            .default_size(140.0)
            .show(ui, |ui| {
                ui.add_space(4.0);
                for (view, label) in [(View::Items, "Items"), (View::Jobs, "Jobs"), (View::About, "About")] {
                    ui.selectable_value(&mut self.view, view, label);
                }
            });
        CentralPanel::default().show(ui, |ui| match self.view {
            View::Items => self.items_view(ui),
            View::Jobs => self.jobs_view(ui),
            View::About => self.about_view(ui),
        });

        for command in std::mem::take(&mut self.pending) {
            self.run(app, command);
        }
        if self.needs_refresh {
            self.refresh(app);
        }
    }

    pub(crate) fn platform(&self) -> &str {
        &self.platform
    }

    pub(crate) fn start_report(&mut self) {
        self.pending.push(Command::StartJob {
            job: JobRequest::GenerateReport {
                step_delay_ms: DEMO_STEP_DELAY_MS,
            },
        });
    }

    fn status_bar(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let text = RichText::new(&self.status.text);
            ui.label(if self.status.is_error {
                text.color(ui.visuals().error_fg_color)
            } else {
                text
            });
            if let Some(info) = &self.info {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.weak(&info.storage);
                });
            }
        });
    }

    fn run(&mut self, app: &mut App, command: Command) {
        let label = command.info().map_or("Command", |info| info.label);
        match app.execute(command) {
            Ok(output) => {
                self.status = Status {
                    text: success_message(label, &output),
                    is_error: false,
                }
            }
            Err(err) => {
                self.status = Status {
                    text: error_message(&err),
                    is_error: true,
                }
            }
        }
        self.needs_refresh = true;
    }

    fn refresh(&mut self, app: &mut App) {
        self.needs_refresh = false;
        let results = [
            app.execute(Command::ListItems),
            app.execute(Command::ListJobs),
            app.execute(Command::Info),
        ];
        for result in results {
            match result {
                Ok(Output::Items(items)) => self.items = items,
                Ok(Output::Jobs(jobs)) => self.jobs = jobs,
                Ok(Output::Info(info)) => self.info = Some(info),
                Ok(_) => {}
                Err(err) => {
                    self.status = Status {
                        text: error_message(&err),
                        is_error: true,
                    }
                }
            }
        }
    }
}

fn success_message(label: &str, output: &Output) -> String {
    match output {
        Output::Item(item) => format!("{label}: '{}'", item.name),
        Output::Deleted { .. } => "Item deleted".to_owned(),
        Output::JobStarted { id } => format!("Job {id} started"),
        Output::CancelRequested { id } => format!("Cancellation of job {id} requested"),
        Output::Items(_) | Output::Jobs(_) | Output::Info(_) => label.to_owned(),
    }
}

fn error_message(err: &AppError) -> String {
    format!("Error ({}): {err}", err.code())
}
