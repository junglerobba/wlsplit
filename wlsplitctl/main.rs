use clap::{App, Arg};
use std::env;
use std::error::Error;
use std::io::prelude::*;
use std::os::unix::net::UnixStream;

const SOCKET_NAME: &str = "wlsplit.sock";

fn main() -> Result<(), Box<dyn Error>> {
    let socket_path = format!(
        "{}/{}",
        env::var("XDG_RUNTIME_DIR").unwrap_or("/tmp".to_string()),
        SOCKET_NAME
    );
    let matches = App::new("wlsplitctl")
        .arg(Arg::with_name("command").required(true).index(1))
        .arg(
            Arg::with_name("socket")
                .short("s")
                .long("socket")
                .default_value(&socket_path),
        )
        .get_matches();

    let socket = matches.value_of("socket").unwrap().to_string();
    let command = matches
        .value_of("command")
        .expect("Input command required!");

    let mut stream = UnixStream::connect(&socket).expect("Server is not running");

    stream.write_all(&command.as_bytes())?;
    Ok(())
}
