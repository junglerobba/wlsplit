use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crossterm::{
    event::EnableMouseCapture, execute, terminal::enable_raw_mode, terminal::EnterAlternateScreen,
};

use livesplit_core::TimeSpan;
use std::{convert::TryInto, error::Error};
use std::{
    io::{stdout, Stdout, Write},
    process,
};

use crate::{wl_split_timer::TimeFormat, wl_split_timer::WlSplitTimer, TimerDisplay};
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    widgets::Row,
    widgets::Table,
    widgets::TableState,
    widgets::{Block, Borders},
    Terminal,
};

use super::{Event, Events};

const TICKRATE: u64 = 10;

const TIMEFORMAT: TimeFormat = TimeFormat {
    hours: 2,
    minutes: 2,
    seconds: 2,
    msecs: 3,
};

pub struct App {
    timer: WlSplitTimer,
    terminal: Terminal<CrosstermBackend<Stdout>>,
    events: Events,
}

impl App {
    pub fn new(timer: WlSplitTimer) -> Self {
        let mut stdout = stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture).unwrap();
        enable_raw_mode().unwrap();

        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.hide_cursor().unwrap();

        let events = Events::new(TICKRATE);
        Self { timer, terminal, events }
    }
}

impl TimerDisplay for App {
    fn run(&mut self) -> Result<(), Box<dyn Error>> {
        let mut rows: Vec<Vec<String>> = Vec::new();
        match self.events.next()? {
            Event::Input(key) => {
                if key == KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)
                    || key == KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE)
                {
                    process::exit(0);
                }
            }
            Event::Tick => {
                for (i, segment) in self.timer.segments().into_iter().enumerate() {
                    let mut row = Vec::new();
                    let index = self.timer.current_segment_index().unwrap_or(0);

                    // Segment
                    if i == index {
                        row.push(format!("> {}", segment.name().to_string()));
                    } else {
                        row.push(format!("  {}", segment.name().to_string()));
                    }

                    // Current
                    if i == index {
                        diff_time(
                            self.timer.time(),
                            segment.personal_best_split_time().real_time,
                            &mut row,
                        );
                    } else if i < index {
                        diff_time(
                            segment.split_time().real_time,
                            self.timer.segments()[i]
                                .personal_best_split_time()
                                .real_time,
                            &mut row,
                        );
                    } else {
                        row.push("".to_string());
                    }

                    // Best
                    if let Some(time) = segment.personal_best_split_time().real_time {
                        row.push(WlSplitTimer::format_time(
                            time.to_duration().num_milliseconds().try_into().unwrap(),
                            TIMEFORMAT,
                            false,
                        ));
                    } else if i == index {
                        if let Some(time) = self.timer.time() {
                            row.push(WlSplitTimer::format_time(
                                time.to_duration().num_milliseconds().try_into().unwrap(),
                                TIMEFORMAT,
                                false,
                            ));
                        }
                    } else if i < index {
                        if let Some(time) = segment.split_time().real_time {
                            row.push(WlSplitTimer::format_time(
                                time.to_duration().num_milliseconds().try_into().unwrap(),
                                TIMEFORMAT,
                                false,
                            ));
                        }
                    } else {
                        row.push("-:--:--.---".to_string());
                    }

                    rows.push(row);
                }

                if let Some(time) = self.timer.time() {
                    let mut row = Vec::new();
                    row.push("".to_string());
                    row.push("".to_string());
                    row.push(WlSplitTimer::format_time(
                        time.to_duration().num_milliseconds().try_into().unwrap(),
                        TIMEFORMAT,
                        false,
                    ));
                    rows.push(row);
                }

                let mut row = Vec::new();
                row.push("".to_string());
                row.push("Sum of best segments".to_string());
                row.push(WlSplitTimer::format_time(
                    self.timer.sum_of_best_segments() as u128,
                    TIMEFORMAT,
                    false,
                ));
                rows.push(row);

                let mut row = Vec::new();
                row.push("".to_string());
                row.push("Best possible time".to_string());
                row.push(WlSplitTimer::format_time(
                    self.timer.best_possible_time() as u128,
                    TIMEFORMAT,
                    false,
                ));
                rows.push(row);
            }
        }
        let title = format!(
            "{} {} - {}",
            self.timer.run().game_name(),
            self.timer.run().category_name(),
            self.timer.run().attempt_count()
        );
        self.terminal.draw(|f| {
            let rects = Layout::default()
                .constraints([Constraint::Percentage(0)].as_ref())
                .margin(5)
                .split(f.size());

            let selected_style = Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD);
            let normal_style = Style::default().fg(Color::White);
            let header = ["Segment", "Current", "Best"];
            let rows = rows.iter().map(|i| Row::StyledData(i.iter(), normal_style));
            let t = Table::new(header.iter(), rows)
                .block(Block::default().borders(Borders::ALL).title(title))
                .highlight_style(selected_style)
                .highlight_symbol(">> ")
                .widths(&[
                    Constraint::Percentage(40),
                    Constraint::Percentage(30),
                    Constraint::Percentage(30),
                ]);
            f.render_stateful_widget(t, rects[0], &mut TableState::default());
        })?;
        Ok(())
    }

    fn split(&mut self) {
        self.timer.split();
    }

    fn start(&mut self) {
        self.timer.start();
    }

    fn pause(&mut self) {
        self.timer.pause();
    }
}

fn diff_time(time: Option<TimeSpan>, best: Option<TimeSpan>, row: &mut Vec<String>) {
    if let (Some(time), Some(best)) = (time, best) {
        let negative: bool;
        let diff: u128;
        let time = time.to_duration().num_milliseconds() as u128;
        let best = best.to_duration().num_milliseconds() as u128;
        if best > time {
            negative = true;
            diff = best - time;
        } else {
            negative = false;
            diff = time - best;
        }
        row.push(WlSplitTimer::format_time(diff, TIMEFORMAT, negative));
    } else {
        row.push("".to_string());
    }
}
