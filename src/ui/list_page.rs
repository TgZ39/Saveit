use egui::scroll_area::ScrollBarVisibility;
use egui::text;
use egui::text::LayoutJob;
use egui::TextFormat;
use egui::{CentralPanel, Context, Grid, TextEdit, Ui};
use egui_extras::DatePickerButton;
use tracing::*;

use crate::database::{handle_delete_source, handle_update_source};
use crate::ui::{set_all_clipboard, set_clipboard, Application, TEXT_INPUT_WIDTH};

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
