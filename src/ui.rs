use std::default::Default;
use std::fmt::{Display, Formatter};
use std::sync::{Arc, RwLock};

use arboard::Clipboard;
use chrono::{Local, NaiveDate};
use eframe::Theme;
use egui::TextStyle::*;
use egui::{CentralPanel, Context, FontFamily, FontId};
use sqlx::SqlitePool;
use tracing::*;

use crate::config::{Config, FormatStandard};
use crate::database::get_all_sources;
use crate::source::Source;

mod start_page;

mod list_page;

mod settings_page;

const TEXT_INPUT_WIDTH: f32 = 450.0;

pub struct Application {
    source_input: SourceInput, // start page
    curr_page: AppPage,
    pub sources_cache: Arc<RwLock<Vec<Source>>>,
    search_query: String,
    edit_modal: EditModal, // edit modal
    settings: Settings,    // settings page
    pub pool: Arc<SqlitePool>,
}

struct EditModal {
    source: Source,
    open: bool,
}

struct SourceInput {
    title: String,
    url: String,
    author: String,
    published_date: NaiveDate,
    published_date_unknown: bool,
    viewed_date: NaiveDate,
    comment: String,
}

struct Settings {
    format_standard: FormatStandard,
    custom_format: String,
}

impl Application {
    fn new(ctx: &Context, pool: Arc<SqlitePool>) -> Self {
        debug!("Creating new Application");
        // make font bigger
        configure_fonts(ctx);

        let config = Config::get_config();

        Self {
            source_input: SourceInput {
                title: String::new(),
                url: String::new(),
                author: String::new(),
                published_date: Local::now().date_naive(),
                published_date_unknown: false,
                viewed_date: Local::now().date_naive(),
                comment: String::new(),
            },
            curr_page: AppPage::Start,
            sources_cache: Arc::new(RwLock::new(vec![])),
            search_query: String::new(),
            edit_modal: EditModal {
                source: Source::default(),
                open: false,
            },
            settings: Settings {
                custom_format: config.custom_format,
                format_standard: config.format_standard,
            },
            pool,
        }
    }

    // get input source from user
    pub fn get_source(&self) -> Source {
        trace!("Reading user source input");

        Source {
            id: -1,
            title: self.source_input.title.clone(),
            url: self.source_input.url.clone(),
            author: self.source_input.url.clone(),
            published_date: self.source_input.published_date,
            viewed_date: self.source_input.viewed_date,
            published_date_unknown: self.source_input.published_date_unknown,
            comment: self.source_input.comment.clone(),
        }
    }

    // clears text fields and reset date to now
    fn clear_input(&mut self) {
        trace!("Clearing user source input");

        self.source_input.title.clear();
        self.source_input.url.clear();
        self.source_input.author.clear();
        self.source_input.published_date = Local::now().date_naive();
        self.source_input.viewed_date = Local::now().date_naive();
        self.source_input.published_date_unknown = false;
        self.source_input.comment.clear();
    }

    fn update_source_cache(&self) {
        trace!("Updating source cache");

        let sources = self.sources_cache.clone();
        let pool = self.pool.clone();

        tokio::task::spawn(async move {
            *sources.write().unwrap() =
                get_all_sources(&pool).await.expect("Error loading sources");
            *sources.write().unwrap() =
                get_all_sources(&pool).await.expect("Error loading sources");
        });
    }
}

pub fn open_gui(pool: Arc<SqlitePool>) -> Result<(), eframe::Error> {
    // set up logging
    env_logger::init();

    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([700.0, 500.0])
        .with_min_inner_size([590.0, 280.0]);

    // load icon
    let icon = eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon.png"));

    if let Ok(icon_data) = icon {
        viewport = viewport.with_icon(icon_data);
    }

    let options = eframe::NativeOptions {
        viewport,
        default_theme: Theme::Dark,
        ..Default::default()
    };

    debug!("Opening GUI");
    // open GUI
    eframe::run_native(
        format!("SaveIt v{}", env!("CARGO_PKG_VERSION")).as_str(),
        options,
        Box::new(|cc| Box::new(Application::new(&cc.egui_ctx, pool))),
    )
}

fn configure_fonts(ctx: &Context) {
    trace!("Configuring fonts");

    let mut style = (*ctx.style()).clone();

    style.text_styles = [
        (Heading, FontId::new(18.0, FontFamily::Proportional)),
        (Body, FontId::new(15.0, FontFamily::Proportional)), // TODO making fontsize above 15 breaks date selection popup
        (Monospace, FontId::new(15.0, FontFamily::Monospace)),
        (Button, FontId::new(15.0, FontFamily::Proportional)),
        (Small, FontId::new(16.0, FontFamily::Proportional)),
    ]
    .into();

    ctx.set_style(style);
}

#[macro_export]
macro_rules! text_label_wrapped {
    ($text:expr, $ui:expr) => {
        let mut job = LayoutJob::single_section($text.to_string(), TextFormat::default());

        job.wrap = text::TextWrapping {
            max_width: 0.0,
            max_rows: 1,
            break_anywhere: true,
            overflow_character: Some('…'),
        };
        $ui.label(job);
    };
}

#[derive(PartialOrd, PartialEq)]
enum AppPage {
    Start,
    List,
    Settings,
}

impl Display for AppPage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AppPage::Start => {
                write!(f, "Start")
            }
            AppPage::List => {
                write!(f, "List")
            }
            AppPage::Settings => {
                write!(f, "Settings")
            }
        }
    }
}

impl eframe::App for Application {
    // runs every frame
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        CentralPanel::default().show(ctx, |ui| {
            // Page selection
            ui.horizontal(|ui| {
                // Start page
                ui.selectable_value(
                    &mut self.curr_page,
                    AppPage::Start,
                    AppPage::Start.to_string(),
                );

                // List page
                let list_page = ui.selectable_value(
                    &mut self.curr_page,
                    AppPage::List,
                    AppPage::List.to_string(),
                );

                if list_page.clicked() {
                    // update source cache
                    self.update_source_cache();
                }

                // Settings page
                ui.selectable_value(
                    &mut self.curr_page,
                    AppPage::Settings,
                    AppPage::Settings.to_string(),
                );
            });

            ui.separator();

            // render selected page
            match self.curr_page {
                AppPage::Start => start_page::render(self, ui),
                AppPage::List => list_page::render(self, ui, ctx),
                AppPage::Settings => settings_page::render(self, ui),
            }
        });
    }
}

pub fn set_clipboard(source: &Source, app: &Application) {
    debug!("Setting clipboard: {:?}", source);

    let mut clipboard = Clipboard::new().unwrap();

    let text = source.format(&app.settings.format_standard);

    clipboard.set_text(text).unwrap();
}

pub fn set_all_clipboard(sources: &[Source], app: &Application) {
    debug!("Setting clipboard with all sources");

    let mut clipboard = Clipboard::new().unwrap();

    let mut text = "".to_string();

    for source in sources {
        text.push_str(source.format(&app.settings.format_standard).as_str());
        text.push('\n');
    }

    clipboard.set_text(text).unwrap();
}
