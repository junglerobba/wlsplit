use crate::display::App as TerminalApp;
use clap::{App, Arg};
use std::error::Error;
use wl_split_timer::WlSplitTimer;

mod display;
mod event;
mod file;
mod wl_split_timer;

pub trait TimerDisplay {
    fn run(&mut self) -> Result<(), Box<dyn Error>>;

    fn split(&mut self);
}

fn main() -> Result<(), Box<dyn Error>> {
    let matches = App::new("wlsplit")
        .arg(Arg::with_name("file").required(true).index(1))
        .arg(
            Arg::with_name("display")
                .short("d")
                .long("display")
                .default_value("terminal"),
        )
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

    if let Some(display) = matches.value_of("display") {
        let mut app = get_app(display, timer)?;
        app.run()?;
    } else {
        panic!()
    }

    Ok(())
}

fn get_app(display: &str, timer: WlSplitTimer) -> Result<impl TimerDisplay, Box<dyn Error>> {
    match display {
        "terminal" => TerminalApp::new(timer),
        _ => {
            panic!("Unknown method");
        }
    }
}
