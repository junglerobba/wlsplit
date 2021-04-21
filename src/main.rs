use crate::display::{Headless, TerminalApp};
use async_trait::async_trait;
use clap::{App, Arg};
use std::{env, error::Error, sync::Arc, time::Duration};
use std::{
    io::{BufRead, BufReader},
    os::unix::net::{UnixListener, UnixStream},
};
use tokio::{sync::Mutex, time::sleep};
use wl_split_timer::WlSplitTimer;
mod display;
mod file;
mod wl_split_timer;

const SOCKET_NAME: &str = "wlsplit.sock";

#[async_trait]
pub trait TimerDisplay: Send + Sync {
    async fn run(&mut self) -> Result<bool, Box<dyn Error>>;

    fn timer(&self) -> &Arc<Mutex<WlSplitTimer>>;
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
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
                .short("c")
                .long("create-file")
                .required(false)
                .takes_value(false),
        )
        .arg(
            Arg::with_name("socket")
                .short("s")
                .long("socket")
                .default_value(&socket_path),
        )
        .get_matches();

    let input = matches.value_of("file").expect("Input file required!");

    let create_file = matches.is_present("create_file");

    let socket = matches.value_of("socket").unwrap().to_string();

    if create_file {
        let timer = WlSplitTimer::new(input.to_string());
        return timer.write_file();
    }

    let timer = WlSplitTimer::from_file(input.to_string());

    let display = matches.value_of("display").unwrap();
    let app = get_app(display, timer);

    let app = Arc::new(Mutex::new(app));
    let timer = Arc::clone(app.lock().await.timer());

    std::fs::remove_file(&socket).ok();
    let listener = UnixListener::bind(&socket).unwrap();
    tokio::spawn(async move {
        for stream in listener.incoming() {
            if let Ok(stream) = stream {
                if handle_stream_response(&timer, stream).await {
                    break;
                }
            }
        }
    });

    loop {
        if app.lock().await.run().await.unwrap_or(false) {
            break;
        }
        sleep(Duration::from_millis(33)).await;
    }
    std::fs::remove_file(&socket).ok();
    Ok(())
}

async fn handle_stream_response(timer: &Arc<Mutex<WlSplitTimer>>, stream: UnixStream) -> bool {
    let stream = BufReader::new(stream);
    for line in stream.lines() {
        match line.unwrap_or_default().as_str() {
            "start" => {
                timer.lock().await.start();
            }
            "split" => {
                timer.lock().await.split();
            }
            "pause" => {
                timer.lock().await.pause();
            }
            "reset" => {
                timer.lock().await.reset(true);
            }
            "quit" => {
                timer.lock().await.quit();
                return true;
            }
            _ => {}
        }
    }
    return false;
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
