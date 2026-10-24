use crate::app::App;

mod app;
mod model;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let mut terminal = ratatui::init();

    let app_result = App::default().run(&mut terminal);

    ratatui::restore();

    app_result
}
