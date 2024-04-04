use egui::{Grid, TextEdit, Ui};
use egui_extras::DatePickerButton;
use tracing::*;

use crate::database::handle_source_save;
use crate::ui::{Application, TEXT_INPUT_WIDTH};

pub fn render(app: &mut Application, ui: &mut Ui) {
    Grid::new("SourceInput").num_columns(2).show(ui, |ui| {
        // input title
        let title_label = ui.label("Title:");
        let input_title =
            TextEdit::singleline(&mut app.source_input.title).desired_width(TEXT_INPUT_WIDTH);
        ui.add(input_title).labelled_by(title_label.id);
        ui.end_row();

        // input URL
        let url_label = ui.label("URL:");
        let input_url =
            TextEdit::singleline(&mut app.source_input.url).desired_width(TEXT_INPUT_WIDTH);
        ui.add(input_url).labelled_by(url_label.id);
        ui.end_row();

        // input author
        let author_label = ui.label("Author:");
        let input_author = TextEdit::singleline(&mut app.source_input.author)
            .hint_text("Leave empty if unknown")
            .desired_width(TEXT_INPUT_WIDTH);
        ui.add(input_author).labelled_by(author_label.id);
        ui.end_row();

        // input published date
        let published_label = ui.label("Date published:");
        ui.horizontal(|ui| {
            ui.add_enabled(
                !app.source_input.published_date_unknown,
                DatePickerButton::new(&mut app.source_input.published_date)
                    .id_source("InputPublishedDate") // needs to be set otherwise the UI would bug with multiple date pickers
                    .show_icon(false),
            )
            .labelled_by(published_label.id);
            ui.checkbox(&mut app.source_input.published_date_unknown, "Unknown");
        });
        ui.end_row();

        // input viewed date
        let viewed_label = ui.label("Date viewed:");
        ui.add(
            DatePickerButton::new(&mut app.source_input.viewed_date)
                .id_source("InputViewedDate") // needs to be set otherwise the UI would bug with multiple date pickers
                .show_icon(false),
        )
        .labelled_by(viewed_label.id);
        ui.end_row();

        // input comment
        let comment_label = ui.label("Comment:");
        let input_comment =
            TextEdit::multiline(&mut app.source_input.comment).desired_width(TEXT_INPUT_WIDTH);
        ui.add(input_comment).labelled_by(comment_label.id);
        ui.end_row();
    });

    ui.add_space(5.0);
    ui.separator();
    ui.add_space(5.0);

    ui.horizontal(|ui| {
        // save input source to DB
        if ui.button("Save").clicked() {
            trace!("Save clicked");
            handle_source_save(app);
        }

        // clear input
        if ui.button("Clear").clicked() {
            trace!("Clear clicked");
            app.clear_input();
        }
    });
}
