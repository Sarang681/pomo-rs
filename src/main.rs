use crate::app::App;

mod alerts;
mod app;
mod digits;
mod input;
mod model;
mod settings;
mod stats;
mod storage;
mod ui;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    // Load before touching the terminal so a broken config is reported on a normal screen.
    let mut app = App::load()?;
    let mut terminal = ratatui::init();

    let app_result = app.run(&mut terminal);

    ratatui::restore();
    for warning in &app.warnings {
        eprintln!("warning: {warning}");
    }

    app_result
}
