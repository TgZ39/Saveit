use crate::database::{delete_source, get_all_sources, insert_source, update_source, Source};
use arboard::Clipboard;
use chrono::{Local, NaiveDate};
use egui::scroll_area::ScrollBarVisibility;
use egui::text::LayoutJob;
use egui::FontFamily::Proportional;
use egui::TextStyle::*;
use egui::{text, Context, FontId, Grid, TextFormat, Ui};
use egui_extras::DatePickerButton;
use futures::executor;
use std::fmt::{Display, Formatter};

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

pub fn open_gui() -> Result<(), eframe::Error> {
    // set up logging
    env_logger::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([500.0, 350.0])
            .with_min_inner_size([500.0, 350.0]),
        ..Default::default()
    };

    // open GUI
    eframe::run_native(
        format!("SaveIt v{}", env!("CARGO_PKG_VERSION")).as_str(),
        options,
        Box::new(|cc| Box::new(Application::new(&cc.egui_ctx))),
    )
}

pub struct Application {
    pub input_url: String,
    pub input_author: String,
    pub input_date: NaiveDate,
    curr_page: AppPage,
    sources_cache: Vec<Source>, // cache needed because every time the user interacted (e.g. mouse movement) with the ui, a new DB request would be made. (30-60/s)
    edit_windows_open: bool, // using cell for more convenient editing of this value (btw fuck the borrow checker)
    edit_source: Source,
}

impl Application {
    fn new(ctx: &Context) -> Self {
        // make font bigger
        configure_fonts(ctx);

        Self {
            input_url: String::new(),
            input_author: String::new(),
            input_date: NaiveDate::from(Local::now().naive_local()), // Current date
            curr_page: AppPage::Start,
            sources_cache: vec![],
            edit_windows_open: false,
            edit_source: Source::default(),
        }
    }

    // get input source from user
    fn get_source(&self) -> Source {
        Source {
            id: -1,
            url: self.input_url.clone(),
            author: self.input_author.clone(),
            date: self.input_date,
        }
    }

    // save input source to DB
    pub fn handle_source_save(&self) {
        // run async fn in sync code ¯\_(ツ)_/¯
        executor::block_on(async {
            let source = self.get_source();

            insert_source(&source)
                .await
                .expect("Error inserting source in database.");
        });
    }

    // clears text fields and reset date to now
    fn clear_input(&mut self) {
        self.input_url.clear();
        self.input_author.clear();
        self.input_date = NaiveDate::from(Local::now().naive_local());
    }

    fn update_source_cache(&mut self) {
        executor::block_on(async {
            self.sources_cache = get_all_sources().await.expect("Error loading sources.");
        });
    }
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
        egui::CentralPanel::default().show(ctx, |ui| {
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
                AppPage::Start => render_start_page(self, ui),
                AppPage::List => render_list_page(self, ui, ctx),
                AppPage::Settings => {}
            }
        });
    }
}

fn render_start_page(app: &mut Application, ui: &mut Ui) {
    Grid::new("SourceInput").num_columns(2).show(ui, |ui| {
        // input URL
        let url_label = ui.label("URL: ");
        ui.text_edit_singleline(&mut app.input_url)
            .labelled_by(url_label.id);
        ui.end_row();

        // input author
        let author_label = ui.label("Author: ");
        ui.text_edit_singleline(&mut app.input_author)
            .labelled_by(author_label.id);
        ui.end_row();

        // input date
        let date_label = ui.label("Date: ");
        ui.add(DatePickerButton::new(&mut app.input_date))
            .labelled_by(date_label.id);
        ui.end_row();
    });

    ui.add_space(10.0);

    ui.horizontal(|ui| {
        // save input source to DB
        if ui.button("Save").clicked() {
            app.handle_source_save();
        }

        // clear input
        if ui.button("Clear").clicked() {
            app.clear_input();
        }
    });
}

fn configure_fonts(ctx: &Context) {
    let mut style = (*ctx.style()).clone();

    style.text_styles = [
        (Heading, FontId::default()),
        (Body, FontId::new(15.0, Proportional)), // TODO making fontsize above 15 breaks date selection popup
        (Monospace, FontId::default()),
        (Button, FontId::default()),
        (Small, FontId::default()),
    ]
    .into();

    ctx.set_style(style);
}

fn render_list_page(app: &mut Application, ui: &mut Ui, ctx: &Context) {
    if ui.button("Copy all").clicked() {
        set_all_clipboard(&app.sources_cache)
    }

    ui.add_space(10.0);

    render_sources(app, ui, ctx);
}

fn render_sources(app: &mut Application, ui: &mut Ui, ctx: &Context) {
    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .drag_to_scroll(true)
        .scroll_bar_visibility(ScrollBarVisibility::AlwaysVisible)
        .show(ui, |ui| {
            for source in app.sources_cache.to_vec() {
                // app.sources_cache.iter().cloned() will NOT work (bug in clippy)
                // source preview
                ui.vertical(|ui| {
                    let id = format!("Index: {}", &source.id);
                    text_label_wrapped!(&id, ui);

                    let url = format!("URL: {}", &source.url);
                    text_label_wrapped!(&url, ui);

                    let author = format!("Author: {}", &source.author);
                    text_label_wrapped!(&author, ui);

                    let date = format!("Date: {}", &source.date.format("%d. %m. %Y"));
                    text_label_wrapped!(&date, ui);
                });

                ui.add_space(5.0);

                // buttons
                ui.horizontal(|ui| {
                    let copy_button = ui.button("Copy");
                    let edit_button = ui.button("Edit");
                    let delete_button = ui.button("Delete");

                    // copy one source
                    if copy_button.clicked() {
                        set_clipboard(&source);
                    }

                    // opens edit modal
                    if edit_button.clicked() {
                        //
                        app.edit_source = source.clone();
                        app.edit_windows_open = true;
                    }

                    let mut update_cache = false;

                    if app.edit_windows_open && app.edit_source.id == source.id {
                        // app.edit_source.id == source.id needed because else it would open an edit model x number of sources in the db

                        // needed because the borrow checker is fucking stupid
                        let mut window_open = true;

                        // edit modal
                        egui::Window::new("Edit source")
                            .collapsible(false)
                            .open(&mut window_open)
                            .show(ctx, |ui| {
                                Grid::new("SourceInput").num_columns(2).show(ui, |ui| {
                                    // input URL
                                    let url_label = ui.label("URL: ");
                                    ui.text_edit_multiline(&mut app.edit_source.url)
                                        .labelled_by(url_label.id);
                                    ui.end_row();

                                    // input author
                                    let author_label = ui.label("Author: ");
                                    ui.text_edit_singleline(&mut app.edit_source.author)
                                        .labelled_by(author_label.id);
                                    ui.end_row();

                                    // input date
                                    let date_label = ui.label("Date: ");
                                    ui.add(DatePickerButton::new(&mut app.edit_source.date))
                                        .labelled_by(date_label.id);
                                    ui.end_row();
                                });

                                ui.add_space(10.0);

                                if ui.button("Save").clicked() {
                                    handle_update_source(app.edit_source.id, &app.edit_source);
                                    update_cache = true;
                                    app.edit_windows_open = false;
                                }
                            });

                        if !window_open {
                            app.edit_windows_open = false;
                        }
                    }

                    if delete_button.clicked() {
                        handle_delete_source(source.id);
                        update_cache = true;
                    }

                    if update_cache {
                        app.update_source_cache();
                    }
                });

                ui.add_space(5.0);
                ui.separator();
                ui.add_space(5.0);
            }
        });
}

fn set_clipboard(source: &Source) {
    let mut clipboard = Clipboard::new().unwrap();

    let text = source.format();

    clipboard.set_text(text).unwrap();
}

fn set_all_clipboard(sources: &Vec<Source>) {
    let mut clipboard = Clipboard::new().unwrap();

    let mut text = "".to_string();

    for source in sources {
        text.push_str(source.format().as_str());
        text.push('\n');
    }

    clipboard.set_text(text).unwrap();
}

fn handle_delete_source(id: i64) {
    executor::block_on(async {
        delete_source(id).await.expect("Error deleting source");
    })
}

fn handle_update_source(id: i64, source: &Source) {
    executor::block_on(async {
        update_source(id, source)
            .await
            .expect("Error deleting source");
    })
}
