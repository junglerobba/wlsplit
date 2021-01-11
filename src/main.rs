use crate::display::TerminalApp;
use async_trait::async_trait;
use clap::{App, Arg};
use std::{error::Error, sync::Arc, time::Duration};
use tokio::{sync::Mutex, time::sleep};
use wl_split_timer::WlSplitTimer;
mod display;
mod file;
mod wl_split_timer;

const SOCKET_PATH: &str = "/tmp/wlsplit.sock";

#[async_trait]
pub trait TimerDisplay: Send + Sync {
    async fn run(&mut self) -> Result<(), Box<dyn Error>>;

    async fn split(&mut self);

    async fn start(&mut self);

    async fn pause(&mut self);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
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
        .get_matches();

    let input = matches.value_of("file").expect("Input file required!");

    let create_file = matches.is_present("create_file");

    let timer = WlSplitTimer::new(input.to_string(), create_file);

    if create_file {
        return timer.write_file();
    }

    let display = matches.value_of("display").unwrap();
    let mut app = get_app(display, timer);

    //app.run().await?;

    let app = Arc::new(Mutex::new(app));

    let cloned = Arc::clone(&app);

    tokio::spawn(async move {
        // cloned.lock().await.start().await;
    });

    let cloned = Arc::clone(&app);
    tokio::spawn(async move {
        loop {
            cloned.lock().await.run().await.unwrap();
        }
    });

    let cloned = Arc::clone(&app);
    tokio::spawn(async move {
        sleep(Duration::from_millis(600)).await;
        cloned.lock().await.start().await;
        sleep(Duration::from_millis(2000)).await;
        cloned.lock().await.split().await;
    });

    loop {}
    Ok(())
}

fn get_app(display: &str, timer: WlSplitTimer) -> Box<dyn TimerDisplay> {
    match display {
        "terminal" => Box::new(TerminalApp::new(timer)),
        _ => {
            panic!("Unknown method");
        }
    }
}
