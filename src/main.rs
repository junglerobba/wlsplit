use clap::{App, Arg};
use std::error::Error;
use wl_split_timer::WlSplitTimer;

mod event;
mod file;
mod terminal;
mod wl_split_timer;

pub trait TimerDisplay {
    fn run(&mut self) -> Result<(), Box<dyn Error>>;

    fn split(&mut self);
}

fn main() -> Result<(), Box<dyn Error>> {
    let matches = App::new("wlsplit")
        .arg(Arg::with_name("file").required(true).index(1))
        .get_matches();

    let input: &str;
    if let Some(file) = matches.value_of("file") {
        input = file;
    } else {
        panic!("Input file required");
    }

    let mut timer = WlSplitTimer::new(input.to_string());

    timer.start();
    timer.pause();

    let mut app = terminal::App::new(timer)?;

    app.run()?;

    Ok(())
}
