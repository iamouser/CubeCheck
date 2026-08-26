use eframe::egui;

pub fn setup_fonts(ctx: &egui::Context) {
    let segoe = r"C:\Windows\Fonts\segoeui.ttf";
    let Ok(bytes) = std::fs::read(segoe) else {
        return;
    };

    let mut fonts = egui::FontDefinitions::empty();
    fonts
        .font_data
        .insert("segoeui".to_owned(), egui::FontData::from_owned(bytes).into());

    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "segoeui".to_owned());
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .insert(0, "segoeui".to_owned());

    ctx.set_fonts(fonts);
}
