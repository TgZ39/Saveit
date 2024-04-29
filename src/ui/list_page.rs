use chrono::{Datelike, NaiveDate};
use egui::scroll_area::ScrollBarVisibility;
use egui::text;
use egui::text::LayoutJob;
use egui::TextFormat;
use egui::{CentralPanel, Context, Grid, TextEdit, Ui};
use egui_extras::DatePickerButton;
use native_dialog::FileDialog;
use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::File;
use std::io::Write;
use tokio::task;
use tracing::*;

use crate::database::{get_all_sources, handle_delete_source, handle_update_source, insert_source};
use crate::source::Source;
use crate::ui::{set_all_clipboard, set_clipboard, Application, TEXT_INPUT_WIDTH};

#[derive(Serialize, Deserialize)]
struct Entry {
    id: i64,
    title: String,
    url: String,
    author: String,
    published_date: i32,
    viewed_date: i32,
    published_date_unknown: bool,
    comment: String,
}

impl From<Source> for Entry {
    fn from(value: Source) -> Self {
        Self {
            id: value.id,
            title: value.title,
            url: value.url,
            author: value.author,
            published_date: value.published_date.num_days_from_ce(),
            viewed_date: value.viewed_date.num_days_from_ce(),
            published_date_unknown: value.published_date_unknown,
            comment: value.comment,
        }
    }
}

#[allow(clippy::from_over_into)]
impl Into<Source> for Entry {
    fn into(self) -> Source {
        Source {
            id: self.id,
            title: self.title,
            url: self.url,
            author: self.author,
            published_date: NaiveDate::from_num_days_from_ce_opt(self.published_date).unwrap(),
            viewed_date: NaiveDate::from_num_days_from_ce_opt(self.viewed_date).unwrap(),
            published_date_unknown: self.published_date_unknown,
            comment: self.comment,
        }
    }
}

pub fn render(app: &mut Application, ui: &mut Ui, ctx: &Context) {
    ui.horizontal(|ui| {
        // Copy all button
        if ui.button("Copy all").clicked() {
            set_all_clipboard(&app.sources_cache.read().unwrap(), app);
        }

        // Search bar
        let input_search = TextEdit::singleline(&mut app.search_query)
            .hint_text("Search")
            .desired_width(TEXT_INPUT_WIDTH);
        ui.add(input_search);

        // Clear button
        if ui.button("Clear").clicked() {
            app.search_query.clear();
        }

        if ui.button("Import").clicked() {
            let path = FileDialog::new()
                .set_location("~")
                .set_title("Select File")
                .add_filter("Json", &["json"])
                .show_open_single_file()
                .unwrap();

            let path = match path {
                None => return,
                Some(path) => path,
            };
            let content = fs::read_to_string(path).expect("Error reading file");
            let entries =
                serde_json::from_str::<Vec<Entry>>(&content).expect("Error deserializing Json");

            let sources = {
                let mut out = Vec::with_capacity(entries.len());
                for entry in entries {
                    out.push(entry.into());
                }
                out
            };

            let pool = app.pool.clone();
            let source_cache = app.sources_cache.clone();

            task::spawn(async move {
                let mut handles = vec![];

                for source in sources {
                    let pool = pool.clone();

                    handles.push(task::spawn(async move {
                        insert_source(&source, &pool)
                            .await
                            .expect("Error saving source");
                    }));
                }

                for handle in handles {
                    handle.await.expect("Error saving source");
                }
                *source_cache.write().unwrap() = get_all_sources(&pool).await.unwrap();
            });
        }

        if ui.button("Export").clicked() {
            let path = FileDialog::new()
                .set_location("~")
                .set_title("Select file")
                .set_filename("export.json")
                .add_filter("Json", &["json"])
                .show_save_single_file()
                .unwrap();

            let path = match path {
                None => return,
                Some(path) => path,
            };
            let mut file = match File::create(path) {
                Ok(f) => f,
                Err(_) => return,
            };

            let sources = app
                .sources_cache
                .read()
                .expect("Error reading source cache");
            let sources = {
                let mut out = Vec::with_capacity(sources.len());
                for source in &*sources {
                    out.push(Entry::from(source.to_owned()))
                }
                out
            };
            let json =
                serde_json::to_string_pretty(&sources).expect("Error converting sources to json");

            file.write_all(json.as_bytes())
                .expect("Error writing to file");
        }
    });

    ui.add_space(10.0);

    render_sources(app, ui, ctx);
}

fn render_sources(app: &mut Application, ui: &mut Ui, ctx: &Context) {
    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .drag_to_scroll(true)
        .scroll_bar_visibility(ScrollBarVisibility::AlwaysVisible)
        .show(ui, |ui| {
            if app.sources_cache.clone().read().unwrap().is_empty() {
                CentralPanel::default().show_inside(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading("Empty");
                    });
                });
                return;
            }

            #[allow(clippy::unnecessary_to_owned)]
            for source in app.sources_cache.clone().read().unwrap().to_vec() {
                if !app.search_query.is_empty() && !source.contains(&app.search_query) {
                    continue;
                }

                // source preview
                ui.vertical(|ui| {
                    let id = format!("Index: {}", &source.id);
                    crate::text_label_wrapped!(&id, ui);

                    let title = format!("Title: {}", &source.title);
                    crate::text_label_wrapped!(&title, ui);

                    let url = format!("URL: {}", &source.url);
                    crate::text_label_wrapped!(&url, ui);

                    let author = format!("Author: {}", &source.author);
                    crate::text_label_wrapped!(&author, ui);

                    let published_date = format!(
                        "Date published: {}",
                        &source.published_date.format("%d. %m. %Y")
                    );
                    if source.published_date_unknown {
                        crate::text_label_wrapped!("Date published: Unknown", ui);
                    } else {
                        crate::text_label_wrapped!(&published_date, ui);
                    }

                    let viewed_date =
                        format!("Date viewed: {}", &source.viewed_date.format("%d. %m. %Y"));
                    crate::text_label_wrapped!(&viewed_date, ui);
                });

                ui.add_space(5.0);

                // buttons
                ui.horizontal(|ui| {
                    let copy_button = ui.button("Copy");
                    let edit_button = ui.button("Edit");
                    let delete_button = ui.button("Delete");

                    // copy one source
                    if copy_button.clicked() {
                        trace!("Copy clicked");
                        set_clipboard(&source, app);
                    }

                    // opens edit modal
                    if edit_button.clicked() {
                        trace!("Edit button clicked");
                        app.edit_modal.source = source.clone();
                        app.edit_modal.open = true;
                    }

                    let mut update_cache = false;

                    if app.edit_modal.open && app.edit_modal.source.id == source.id {
                        // app.edit_source.id == source.id needed because else it would open an edit model x number of sources in the db

                        // needed because the borrow checker is fucking stupid
                        let mut window_open = true;

                        // edit modal
                        egui::Window::new("Edit source")
                            .auto_sized()
                            .resizable(true)
                            .collapsible(false)
                            .open(&mut window_open)
                            .show(ctx, |ui| {
                                Grid::new("SourceInput").num_columns(2).show(ui, |ui| {
                                    // input title
                                    let title_label = ui.label("Title:");
                                    let input_title =
                                        TextEdit::singleline(&mut app.edit_modal.source.title)
                                            .desired_width(TEXT_INPUT_WIDTH);
                                    ui.add(input_title).labelled_by(title_label.id);
                                    ui.end_row();

                                    // input URL
                                    let url_label = ui.label("URL:");
                                    let input_url =
                                        TextEdit::singleline(&mut app.edit_modal.source.url)
                                            .desired_width(TEXT_INPUT_WIDTH);
                                    ui.add(input_url).labelled_by(url_label.id);
                                    ui.end_row();

                                    // input author
                                    let author_label = ui.label("Author:");
                                    let input_author =
                                        TextEdit::singleline(&mut app.edit_modal.source.author)
                                            .hint_text("Leave empty if unknown")
                                            .desired_width(TEXT_INPUT_WIDTH);
                                    ui.add(input_author).labelled_by(author_label.id);
                                    ui.end_row();

                                    // input published date
                                    let published_label = ui.label("Date published:");
                                    ui.horizontal(|ui| {
                                        ui.add_enabled(
                                            !app.edit_modal.source.published_date_unknown,
                                            DatePickerButton::new(
                                                &mut app.edit_modal.source.published_date,
                                            )
                                            .id_source("InputPublishedDate") // needs to be set otherwise the UI would bug with multiple date pickers
                                            .show_icon(false),
                                        )
                                        .labelled_by(published_label.id);
                                        ui.checkbox(
                                            &mut app.edit_modal.source.published_date_unknown,
                                            "Unknown",
                                        );
                                    });
                                    ui.end_row();

                                    // input viewed date
                                    let viewed_label = ui.label("Date viewed:");
                                    ui.add(
                                        DatePickerButton::new(
                                            &mut app.edit_modal.source.viewed_date,
                                        )
                                        .id_source("InputViewedDate") // needs to be set otherwise the UI would bug with multiple date pickers
                                        .show_icon(false),
                                    )
                                    .labelled_by(viewed_label.id);
                                    ui.end_row();

                                    // input comment
                                    let comment_label = ui.label("Comment:");
                                    let input_comment =
                                        TextEdit::multiline(&mut app.edit_modal.source.comment)
                                            .desired_width(TEXT_INPUT_WIDTH);
                                    ui.add(input_comment).labelled_by(comment_label.id);
                                    ui.end_row();
                                });

                                ui.add_space(10.0);

                                if ui.button("Save").clicked() {
                                    trace!("Edit modal save clicked");
                                    handle_update_source(
                                        app.edit_modal.source.id,
                                        &app.edit_modal.source,
                                        app,
                                    );
                                    update_cache = true;
                                    app.edit_modal.open = false;
                                }
                            });

                        if !window_open {
                            app.edit_modal.open = false;
                        }
                    }

                    if delete_button.clicked() {
                        trace!("Delete clicked");
                        handle_delete_source(source.id, app);
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
