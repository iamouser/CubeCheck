use eframe::egui;

fn system_font_paths() -> &'static [&'static str] {
    #[cfg(windows)]
    {
        &[
            r"C:\Windows\Fonts\segoeui.ttf",
            r"C:\Windows\Fonts\arial.ttf",
            r"C:\Windows\Fonts\tahoma.ttf",
        ]
    }
    #[cfg(not(windows))]
    {
        &[
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
            "/usr/share/fonts/TTF/DejaVuSans.ttf",
            "/System/Library/Fonts/SFNS.ttf",
            "/System/Library/Fonts/Helvetica.ttc",
        ]
    }
}

pub fn setup_fonts(ctx: &egui::Context) {
    let Some((name, bytes)) = system_font_paths().iter().find_map(|path| {
        std::fs::read(path)
            .ok()
            .map(|bytes| (path.rsplit(['/', '\\']).next().unwrap_or("ui").to_owned(), bytes))
    }) else {
        return;
    };

    let mut fonts = egui::FontDefinitions::empty();
    fonts
        .font_data
        .insert(name.clone(), egui::FontData::from_owned(bytes).into());

    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, name.clone());
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .insert(0, name);

    ctx.set_fonts(fonts);
}
