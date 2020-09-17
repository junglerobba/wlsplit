use std::error::Error;
use wl_split_timer::WlSplitTimer;

mod event;
mod terminal;
mod wl_split_timer;

pub trait TimerDisplay {
    fn run(&mut self) -> Result<(), Box<dyn Error>>;

    fn split(&mut self);
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut timer = WlSplitTimer::new("".to_string());

    timer.start();
    timer.pause();

    let mut app = terminal::App::new(timer)?;

    app.run()?;

    Ok(())
}
