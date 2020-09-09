use std::error::Error;
use wl_split_timer::WlSplitTimer;

mod event;
mod terminal;
mod wl_split_timer;

fn main() -> Result<(), Box<dyn Error>> {
    let mut timer = WlSplitTimer::new("".to_string());

    timer.start();
    timer.pause();

    let app = terminal::App::new(timer)?;

    app.run()?;

    Ok(())
}
