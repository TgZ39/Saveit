use egui::{ComboBox, TextEdit, Ui};
use tracing::*;

use crate::config::{Config, FormatStandard};
use crate::ui::{Application, TEXT_INPUT_WIDTH};

pub fn render(app: &mut Application, ui: &mut Ui) {
    // select source formatting standard
    ComboBox::from_label("Select source format")
        .selected_text(format!("{:?}", app.settings.format_standard))
        .show_ui(ui, |ui| {
            ui.selectable_value(
                &mut app.settings.format_standard,
                FormatStandard::Default,
                "Default",
            );
            ui.selectable_value(
                &mut app.settings.format_standard,
                FormatStandard::Custom,
                "Custom",
            );
        });

    ui.horizontal(|ui| {
        let custom_label = ui.label("Custom format:");
        let input_custom_format =
            TextEdit::singleline(&mut app.settings.custom_format).desired_width(TEXT_INPUT_WIDTH);

        #[allow(clippy::match_like_matches_macro)] // clippy complaining again LOL
        let custom_format_enabled = match app.settings.format_standard {
            FormatStandard::Custom => true,
            _ => false,
        };
        ui.add_enabled(custom_format_enabled, input_custom_format)
            .labelled_by(custom_label.id);
    });

    ui.add_space(5.0);
    ui.separator();
    ui.add_space(5.0);

    // Save button
    if ui.button("Save").clicked() {
        trace!("Save clicked");
        let mut config = Config::get_config();

        // Source formatting standard
        config.format_standard = app.settings.format_standard.clone();

        // Custom format
        config.custom_format = app.settings.custom_format.clone();

        config.save();
    }
}
