use eframe::egui;
use starter_application::{App, SystemClock, jobs::InlineJobRunner};
use starter_platform::web::{init_logging, request_persistent_storage};
use starter_storage_browser::BrowserItemRepository;
use starter_ui_egui::Shell;
use wasm_bindgen::JsCast;

const CANVAS_ID: &str = "app_canvas";
const DB_NAME: &str = "rust-app";

struct WebApp {
    shell: Shell,
    app: App,
}

impl eframe::App for WebApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.shell.logic(ctx, &mut self.app);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.shell.ui(ui, &mut self.app);
    }
}

pub fn start() {
    init_logging(log::LevelFilter::Info);
    wasm_bindgen_futures::spawn_local(async {
        if let Err(message) = run().await {
            log::error!("{message}");
            show_fatal_error(&message);
        }
    });
}

async fn run() -> Result<(), String> {
    if !request_persistent_storage().await {
        log::info!("persistent storage not granted; the browser may evict data under storage pressure");
    }
    // Load stored data before the UI starts, so the first frame already shows it.
    let repo = BrowserItemRepository::open_indexed_db(DB_NAME)
        .await
        .map_err(|e| format!("storage: {e}"))?;
    let app = App::new(
        Box::new(repo),
        Box::new(InlineJobRunner::default()),
        Box::new(SystemClock),
    );

    let canvas = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id(CANVAS_ID))
        .and_then(|e| e.dyn_into::<web_sys::HtmlCanvasElement>().ok())
        .ok_or_else(|| format!("canvas #{CANVAS_ID} not found"))?;

    eframe::WebRunner::new()
        .start(
            canvas,
            eframe::WebOptions::default(),
            Box::new(move |_cc| {
                Ok(Box::new(WebApp {
                    shell: Shell::new("Browser (WebAssembly)"),
                    app,
                }))
            }),
        )
        .await
        .map_err(|e| format!("eframe failed to start: {e:?}"))?;

    if let Some(loading) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id("loading"))
    {
        loading.remove();
    }
    Ok(())
}

fn show_fatal_error(message: &str) {
    if let Some(element) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id("loading"))
    {
        element.set_text_content(Some(&format!("The app could not start: {message}")));
    }
}
