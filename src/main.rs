use crate::{
    display::{Headless, TerminalApp},
    wl_split_timer::RunMetadata,
};
use clap::{App, Arg};
use std::{
    env,
    error::Error,
    fs::OpenOptions,
    sync::{Arc, Mutex},
    time::Duration,
};
use std::{
    io::{BufRead, BufReader},
    os::unix::net::{UnixListener, UnixStream},
};
use wl_split_timer::WlSplitTimer;
mod display;
mod file;
mod time_format;
mod wl_split_timer;

const SOCKET_NAME: &str = "wlsplit.sock";

pub trait TimerDisplay {
    fn run(&mut self) -> Result<bool, Box<dyn Error>>;

    fn timer(&self) -> &Arc<Mutex<WlSplitTimer>>;
}

fn main() -> Result<(), Box<dyn Error>> {
    let socket_path = format!(
        "{}/{}",
        env::var("XDG_RUNTIME_DIR").unwrap_or("/tmp".to_string()),
        SOCKET_NAME
    );
    let matches = App::new("wlsplit")
        .arg(Arg::with_name("file").required(true).index(1))
        .arg(
            Arg::with_name("display")
                .short("d")
                .long("display")
                .default_value("terminal"),
        )
        .arg(
            Arg::with_name("create_file")
                .short("f")
                .long("create-file")
                .long_help("Creates a new file regardless if a file already exists in that location or not")
                .required(false)
                .takes_value(false),
        )
        .arg(
            Arg::with_name("game_name")
                .long_help("Game name to use when generating run file")
                .long("game")
                .required(false)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("category_name")
                .long_help("Category name to use when generating run file")
                .long("category")
                .required(false)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("splits")
                .long_help("Comma separated list of splits to use when generating run file")
                .long("splits")
                .required(false)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("socket")
                .short("s")
                .long("socket")
                .default_value(&socket_path),
        )
        .get_matches();

    let input = matches.value_of("file").expect("Input file required!");

    let create_file = matches.is_present("create_file")
        || OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(input)
            .is_ok();

    let socket = matches.value_of("socket").unwrap().to_string();

    let timer = if create_file {
        let metadata = RunMetadata {
            game_name: matches.value_of("game_name"),
            category_name: matches.value_of("category_name"),
            splits: matches
                .value_of("splits")
                .map(|split_names| split_names.split(',').collect()),
        };
        WlSplitTimer::new(input.to_string(), metadata)
    } else {
        WlSplitTimer::from_file(input.to_string())
    };

    let display = matches.value_of("display").unwrap();
    let app = get_app(display, timer);

    let app = Arc::new(Mutex::new(app));
    let timer = Arc::clone(app.lock().unwrap().timer());

    std::fs::remove_file(&socket).ok();
    let listener = UnixListener::bind(&socket).unwrap();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            if handle_stream_response(&timer, stream) {
                break;
            }
        }
    });

    loop {
        if app.lock().unwrap().run().unwrap_or(false) {
            break;
        }
        std::thread::sleep(Duration::from_millis(33));
    }
    std::fs::remove_file(&socket).ok();
    Ok(())
}

fn handle_stream_response(timer: &Arc<Mutex<WlSplitTimer>>, stream: UnixStream) -> bool {
    let stream = BufReader::new(stream);
    for line in stream.lines() {
        match line.unwrap_or_default().as_str() {
            "start" => {
                timer.lock().unwrap().start();
            }
            "split" => {
                timer.lock().unwrap().split();
            }
            "skip" => {
                timer.lock().unwrap().skip();
            }
            "pause" => {
                timer.lock().unwrap().pause();
            }
            "reset" => {
                timer.lock().unwrap().reset(true);
            }
            "quit" => {
                timer.lock().unwrap().quit();
                return true;
            }
            _ => {}
        }
    }
    false
}

fn get_app(display: &str, timer: WlSplitTimer) -> Box<dyn TimerDisplay> {
    match display {
        "terminal" => Box::new(TerminalApp::new(timer)),
        "null" => Box::new(Headless::new(timer)),
        _ => {
            panic!("Unknown method");
        }
    }
}
