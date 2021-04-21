mod terminal;

pub use self::terminal::App as TerminalApp;

mod null;

pub use self::null::App as Headless;