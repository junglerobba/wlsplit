use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
};

use crate::{time_format::TimeFormat, wl_split_timer::WlSplitTimer, TimerDisplay};
use livesplit_core::TimeSpan;
use std::io::{stdout, Stdout};
use std::{
    convert::TryInto,
    error::Error,
    sync::{Arc, Mutex},
};
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

pub struct App {
    timer: Arc<Mutex<WlSplitTimer>>,
    terminal: Terminal<CrosstermBackend<Stdout>>,
}
impl App {
    pub fn new(timer: WlSplitTimer) -> Self {
        let mut stdout = stdout();
        execute!(stdout, EnterAlternateScreen).unwrap();

        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.hide_cursor().unwrap();

        Self {
            timer: Arc::new(Mutex::new(timer)),
            terminal,
        }
    }

    fn quit(&mut self) {
        execute!(stdout(), LeaveAlternateScreen).unwrap();
        self.terminal.show_cursor().unwrap();
    }
}

impl TimerDisplay for App {
    fn run(&mut self) -> Result<bool, Box<dyn Error>> {
        let mut rows: Vec<Vec<String>> = Vec::new();

        let timer = self.timer.lock().unwrap();
        if timer.exit {
            drop(timer);
            self.quit();
            return Ok(true);
        }
        for (i, segment) in timer.segments().iter().enumerate() {
            let mut row = Vec::new();
            let index = timer.current_segment_index().unwrap_or(0);

            // Segment
            if i == index {
                row.push(format!("> {}", segment.name().to_string()));
            } else {
                row.push(format!("  {}", segment.name().to_string()));
            }

            // Current
            row.push(match i.cmp(&index) {
                std::cmp::Ordering::Equal => {
                    diff_time(timer.time(), segment.personal_best_split_time().real_time)
                }
                std::cmp::Ordering::Less => diff_time(
                    segment.split_time().real_time,
                    timer.segments()[i].personal_best_split_time().real_time,
                ),
                _ => "".to_string(),
            });

            let time = if let Some(time) = segment.personal_best_split_time().real_time {
                Some(time)
            } else if segment.segment_history().iter().len() == 0 {
                segment.split_time().real_time
            } else {
                None
            };
            row.push(time.map_or("-:--:--.---".to_string(), |time| {
                TimeFormat::default()
                    .format_time(time.to_duration().num_milliseconds() as u128, false)
            }));

            rows.push(row);
        }

        if let Some(time) = timer.time() {
            rows.push(vec![
                "".to_string(),
                "".to_string(),
                TimeFormat::default().format_time(
                    time.to_duration().num_milliseconds().try_into().unwrap(),
                    false,
                ),
            ]);
        }

        rows.push(vec![
            "".to_string(),
            "Sum of best segments".to_string(),
            TimeFormat::default().format_time(timer.sum_of_best_segments() as u128, false),
        ]);

        rows.push(vec![
            "".to_string(),
            "Best possible time".to_string(),
            TimeFormat::default().format_time(timer.best_possible_time() as u128, false),
        ]);

        let title = format!(
            "{} {} - {}",
            timer.run().game_name(),
            timer.run().category_name(),
            timer.run().attempt_count()
        );

        drop(timer);

        self.terminal.draw(|f| {
            let rects = Layout::default()
                .constraints([Constraint::Percentage(0)].as_ref())
                .margin(0)
                .split(f.size());

            let selected_style = Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD);
            let normal_style = Style::default().fg(Color::White);
            let header = ["Segment", "Current", "Best"];
            let rows = rows.iter().map(|i| Row::StyledData(i.iter(), normal_style));
            let t = Table::new(header.iter(), rows)
                .block(Block::default().borders(Borders::NONE).title(title))
                .highlight_style(selected_style)
                .highlight_symbol(">> ")
                .widths(&[
                    Constraint::Percentage(40),
                    Constraint::Percentage(30),
                    Constraint::Percentage(30),
                ]);
            f.render_stateful_widget(t, rects[0], &mut TableState::default());
        })?;
        Ok(false)
    }

    fn timer(&self) -> &Arc<Mutex<WlSplitTimer>> {
        &self.timer
    }
}
fn diff_time(time: Option<TimeSpan>, best: Option<TimeSpan>) -> String {
    if let (Some(time), Some(best)) = (time, best) {
        let time = time.to_duration().num_milliseconds();
        let best = best.to_duration().num_milliseconds();
        let negative = best > time;
        let diff = if negative { best - time } else { time - best } as u128;
        return TimeFormat::for_diff().format_time(diff, negative);
    }
    "".to_string()
}
