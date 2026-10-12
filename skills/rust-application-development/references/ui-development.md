# UI development

## Principles

- A UI crate depends on the application core and its framework only.
- It holds **view state** (selected tab, form drafts, confirmation flags) and **caches of outputs**
  (lists it renders). Domain state lives in `App`.
- User actions become `Command`s. In immediate-mode UIs, queue them while drawing
  (`self.pending.push(cmd)`) and execute after layout – avoids borrow conflicts and double execution.
- Refresh caches when `drain_events()` returns something (or after own commands).
- Keep frames coming while jobs run (`ctx.request_repaint_after(16ms)`), otherwise stop repainting.
- Errors from `execute` go to a visible status area with the error code; never panic in UI code.
- Destructive actions require an explicit confirmation step in the UI.

## egui / eframe (0.36)

- `eframe::App` has `logic(&mut self, ctx, frame)` (no drawing: poll jobs, process events) and
  `ui(&mut self, ui, frame)` (draw). Panels: `egui::Panel::{top, bottom, left, right}(id).show(ui, …)`,
  `egui::CentralPanel::default().show(ui, …)`.
- The UI crate should depend on `egui`, not `eframe`; hosts pick native (`wgpu`) or web (`glow`/`wgpu`)
  rendering.
- Accessibility labels: `ui.add(TextEdit::singleline(..)).labelled_by(label.id)` – also makes
  `egui_kittest` queries work.
- Theme switching: `egui::widgets::global_theme_preference_switch(ui)`.
- Background threads wake the UI with a cloned `egui::Context::request_repaint()` (pass it to the
  job runner as notifier).
- Test with `egui_kittest::Harness::new_ui_state(|ui, state| …, state)`, `harness.run()`,
  `get_by_label(..).click()`, `.focus()` then `.type_text(..)`.

## Adding another framework

1. New crate `ui-<framework>` at the UI layer; never a shared "widget abstraction".
2. Map: user action → `Command`; `Output` → view model; `AppEvent` → refresh/notification; a timer or
   subscription calls `App::poll()` while jobs run.
3. Host in `apps/<name>` wires the same adapters as the existing desktop host.

| Framework | Where `App` lives | Notes |
|---|---|---|
| Iced | application state | commands from `update(message)`; `Subscription::run`/timer for polling |
| Slint | Rust side behind `Rc<RefCell<App>>` | callbacks execute commands; models from outputs |
| Dioxus | context provider (`Rc<RefCell<App>>`) | signals updated from events; desktop + web |
| Tauri | Rust state behind `Mutex<App>` | `#[tauri::command] fn execute(cmd: Command)`; JSON from the web frontend; `emit` events |

## Don'ts

- Calling repositories, `std::fs`, `std::thread` from UI code.
- Formatting rules that are business rules (validation belongs in the domain; the UI only displays errors).
- Blocking the UI thread for more than a few milliseconds – use a job.
