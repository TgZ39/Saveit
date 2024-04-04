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
    pub input_title: String,
    pub input_url: String,
    pub input_author: String,
    pub input_published_date: NaiveDate,
    input_published_disabled: bool,
    pub input_viewed_date: NaiveDate,
    pub input_comment: String,
    curr_page: AppPage,
    pub sources_cache: Arc<RwLock<Vec<Source>>>,
    // cache needed because every time the user interacted (e.g. mouse movement) with the ui, a new DB request would be made. (30-60/s)
    edit_windows_open: bool,
    // using cell for more convenient editing of this value (btw fuck the borrow checker)
    edit_source: Source,
    input_format_standard: FormatStandard,
    input_custom_format: String,
    search_query: String,
    pub db_pool: Arc<SqlitePool>,
}

impl Application {
    fn new(ctx: &Context, pool: SqlitePool) -> Self {
        debug!("Creating new Application");
        // make font bigger
        configure_fonts(ctx);

        let config = Config::get_config();

        Self {
            input_title: String::new(),
            input_url: String::new(),
            input_author: String::new(),
            input_published_date: NaiveDate::from(Local::now().naive_local()), // Current date
            input_published_disabled: false,
            input_viewed_date: NaiveDate::from(Local::now().naive_local()), // Current date
            input_comment: String::new(),
            curr_page: AppPage::Start,
            sources_cache: Arc::new(RwLock::new(vec![])),
            edit_windows_open: false,       // edit modal
            edit_source: Source::default(), // source to edit in the edit modal
            input_format_standard: config.format_standard,
            input_custom_format: config.custom_format,
            search_query: String::new(),
            db_pool: Arc::new(pool),
        }
    }

    // get input source from user
    pub(crate) fn get_source(&self) -> Source {
        trace!("Reading user source input");

        Source {
            id: -1,
            title: self.input_title.clone(),
            url: self.input_url.clone(),
            author: self.input_author.clone(),
            published_date: self.input_published_date,
            viewed_date: self.input_viewed_date,
            published_date_unknown: self.input_published_disabled,
            comment: self.input_comment.clone(),
        }
    }

    // clears text fields and reset date to now
    fn clear_input(&mut self) {
        trace!("Clearing user source input");

        self.input_title.clear();
        self.input_url.clear();
        self.input_author.clear();
        self.input_published_date = NaiveDate::from(Local::now().naive_local());
        self.input_viewed_date = NaiveDate::from(Local::now().naive_local());
        self.input_published_disabled = false;
        self.input_comment.clear();
    }

    fn update_source_cache(&self) {
        trace!("Updating source cache");

        let sources = self.sources_cache.clone();
        let pool = self.db_pool.clone();

        tokio::task::spawn(async move {
            *sources.write().unwrap() =
                get_all_sources(&pool).await.expect("Error loading sources");
        });
    }
}

pub fn open_gui(pool: SqlitePool) -> Result<(), eframe::Error> {
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

    let text = source.format(&app.input_format_standard);

    clipboard.set_text(text).unwrap();
}

pub fn set_all_clipboard(sources: &[Source], app: &Application) {
    debug!("Setting clipboard with all sources");

    let mut clipboard = Clipboard::new().unwrap();

    let mut text = "".to_string();

    for source in sources {
        text.push_str(source.format(&app.input_format_standard).as_str());
        text.push('\n');
    }

    clipboard.set_text(text).unwrap();
}
