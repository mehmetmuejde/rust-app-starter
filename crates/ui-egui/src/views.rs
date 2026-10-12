//! The three content views. They only read the shell's caches and queue commands.

use egui::{Key, ProgressBar, RichText, ScrollArea, TextEdit};
use starter_application::{
    Command,
    jobs::{JobOutput, JobState},
};

use crate::shell::Shell;

impl Shell {
    pub(crate) fn items_view(&mut self, ui: &mut egui::Ui) {
        ui.heading("Items");
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            let label = ui.label("Name:");
            let edit = ui
                .add(
                    TextEdit::singleline(&mut self.new_name)
                        .hint_text("e.g. Groceries")
                        .desired_width(240.0),
                )
                .labelled_by(label.id);
            let submitted = edit.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
            if ui.button("Create").clicked() || submitted {
                self.pending.push(Command::CreateItem {
                    name: std::mem::take(&mut self.new_name),
                });
                edit.request_focus();
            }
        });
        ui.separator();

        if self.items.is_empty() {
            ui.weak("No items yet. Create one above, or from the CLI: rust-app create \"My item\"");
            return;
        }
        ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            egui::Grid::new("items")
                .striped(true)
                .num_columns(3)
                .spacing([16.0, 6.0])
                .show(ui, |ui| {
                    ui.strong("Name");
                    ui.strong("Created (UTC)");
                    ui.strong("");
                    ui.end_row();
                    for item in &self.items {
                        match &mut self.editing {
                            Some((id, draft)) if *id == item.id => {
                                ui.add(TextEdit::singleline(draft).desired_width(200.0));
                            }
                            _ => {
                                ui.label(item.name.as_str());
                            }
                        }
                        ui.label(item.created_at.to_rfc3339());
                        ui.horizontal(|ui| {
                            let editing_this = self.editing.as_ref().is_some_and(|(id, _)| *id == item.id);
                            if editing_this {
                                if ui.button("Save").clicked()
                                    && let Some((id, name)) = self.editing.take()
                                {
                                    self.pending.push(Command::RenameItem { id, name });
                                }
                                if ui.button("Cancel").clicked() {
                                    self.editing = None;
                                }
                            } else if self.confirm_delete == Some(item.id) {
                                ui.label("Delete?");
                                if ui.button("Yes").clicked() {
                                    self.pending.push(Command::DeleteItem { id: item.id });
                                    self.confirm_delete = None;
                                }
                                if ui.button("No").clicked() {
                                    self.confirm_delete = None;
                                }
                            } else {
                                if ui.button("Rename").clicked() {
                                    self.editing = Some((item.id, item.name.to_string()));
                                }
                                if ui.button("Delete").clicked() {
                                    self.confirm_delete = Some(item.id);
                                }
                            }
                        });
                        ui.end_row();
                    }
                });
        });
    }

    pub(crate) fn jobs_view(&mut self, ui: &mut egui::Ui) {
        ui.heading("Background jobs");
        ui.label("Jobs run outside the UI: on worker threads natively, in small slices per frame in the browser.");
        ui.add_space(4.0);
        if ui.button("Generate report").clicked() {
            self.start_report();
        }
        ui.separator();
        if self.jobs.is_empty() {
            ui.weak("No jobs yet.");
            return;
        }
        ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            for job in self.jobs.iter().rev() {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("#{} {}", job.id, job.label)).strong());
                    match &job.state {
                        JobState::Queued => {
                            ui.label("queued");
                        }
                        JobState::Running { done, total } => {
                            let fraction = if *total == 0 { 0.0 } else { *done as f32 / *total as f32 };
                            ui.add(
                                ProgressBar::new(fraction)
                                    .desired_width(200.0)
                                    .text(format!("{done}/{total}")),
                            );
                        }
                        JobState::Succeeded { output } => {
                            ui.label(describe_output(output));
                        }
                        JobState::Failed { error } => {
                            ui.colored_label(ui.visuals().error_fg_color, format!("failed: {error}"));
                        }
                        JobState::Cancelled => {
                            ui.weak("cancelled");
                        }
                    }
                    if !job.state.is_finished() && ui.button("Cancel").clicked() {
                        self.pending.push(Command::CancelJob { id: job.id });
                    }
                });
            }
        });
    }

    pub(crate) fn about_view(&mut self, ui: &mut egui::Ui) {
        ui.heading("About");
        egui::Grid::new("about")
            .num_columns(2)
            .spacing([16.0, 6.0])
            .show(ui, |ui| {
                ui.label("Version");
                ui.label(env!("CARGO_PKG_VERSION"));
                ui.end_row();
                ui.label("Platform");
                ui.label(self.platform());
                ui.end_row();
                if let Some(info) = &self.info {
                    ui.label("Storage");
                    ui.label(&info.storage);
                    ui.end_row();
                    ui.label("Items");
                    ui.label(info.item_count.to_string());
                    ui.end_row();
                }
            });
        ui.add_space(8.0);
        ui.label("Every button here runs a typed command through App::execute — the same path the CLI uses.");
    }
}

fn describe_output(output: &JobOutput) -> String {
    match output {
        JobOutput::Report {
            item_count,
            total_characters,
            longest_name,
        } => format!(
            "{item_count} items, {total_characters} characters, longest: {}",
            longest_name.as_deref().unwrap_or("–")
        ),
    }
}

#[cfg(test)]
mod tests {
    use egui_kittest::{Harness, kittest::Queryable};
    use starter_application::{App, InMemoryItemRepository, ManualClock, Output, jobs::InlineJobRunner};
    use starter_domain::Timestamp;

    use super::*;

    struct State {
        shell: Shell,
        app: App,
    }

    fn harness() -> Harness<'static, State> {
        let app = App::new(
            Box::new(InMemoryItemRepository::new()),
            Box::new(InlineJobRunner::default()),
            Box::new(ManualClock::new(Timestamp::from_millis(0))),
        );
        let state = State {
            shell: Shell::new("Test"),
            app,
        };
        Harness::new_ui_state(
            |ui, state: &mut State| {
                state.shell.logic(ui.ctx(), &mut state.app);
                state.shell.ui(ui, &mut state.app);
            },
            state,
        )
    }

    #[test]
    fn create_item_through_the_ui() {
        let mut harness = harness();
        harness.run();
        harness.get_by_label("Name:").focus();
        harness.run();
        harness.get_by_label("Name:").type_text("Milk");
        harness.run();
        harness.get_by_label("Create").click();
        harness.run();

        let Output::Items(items) = harness.state_mut().app.execute(Command::ListItems).unwrap() else {
            panic!("expected items")
        };
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name.as_str(), "Milk");
        assert!(harness.query_by_label("Milk").is_some(), "new item is listed");
    }

    #[test]
    fn invalid_name_shows_error_in_status_bar() {
        let mut harness = harness();
        harness.run();
        harness.get_by_label("Create").click();
        harness.run();
        assert!(harness.query_by_label_contains("Error (validation)").is_some());
    }

    #[test]
    fn about_view_shows_platform() {
        let mut harness = harness();
        harness.run();
        harness.get_by_label("About").click();
        harness.run();
        assert!(harness.query_by_label("Test").is_some());
    }
}
