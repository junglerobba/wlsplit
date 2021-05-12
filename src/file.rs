use std::{error::Error, fs::File, io::Read, io::Write};

use livesplit_core::Run as LivesplitRun;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::time_format::TimeFormat;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Run {
    pub game_name: String,
    pub category_name: String,
    pub attempt_count: usize,
    pub attempt_history: Vec<Attempt>,
    pub segments: Vec<Segment>,
}

impl Default for Run {
    fn default() -> Self {
        let segments = vec![Segment {
            name: "Example Segment".to_string(),
            ..Default::default()
        }];

        Self {
            game_name: "Example Splits".to_string(),
            category_name: "Any%".to_string(),
            attempt_count: 0,
            attempt_history: Vec::new(),
            segments,
        }
    }
}

impl Run {
    pub fn new(run: &LivesplitRun) -> Self {
        let mut attempt_history: Vec<Attempt> = Vec::new();
        for attempt in run.attempt_history() {
            if let Some(time) = attempt.time().real_time {
                attempt_history.push(Attempt {
                    time: Some(
                        TimeFormat::for_file()
                            .format_time(time.total_milliseconds() as u128, false),
                    ),
                    id: attempt.index(),
                    started: attempt.started().map(|t| t.time.to_rfc3339()),
                    ended: attempt.ended().map(|t| t.time.to_rfc3339()),
                    pause_time: attempt.pause_time().map(|t| {
                        TimeFormat::for_file().format_time(t.total_milliseconds() as u128, false)
                    }),
                });
            }
        }

        let mut segments: Vec<Segment> = Vec::new();
        for segment in run.segments() {
            let best_segment_time = segment.best_segment_time().real_time.map(|time| {
                TimeFormat::for_file().format_time(time.total_milliseconds() as u128, false)
            });

            let personal_best_split_time =
                segment.personal_best_split_time().real_time.map(|time| {
                    TimeFormat::for_file().format_time(time.total_milliseconds() as u128, false)
                });

            let segment_history: Vec<SplitTime> = segment
                .segment_history()
                .iter()
                .map(|entry| SplitTime {
                    id: Some(entry.0),
                    time: entry.1.real_time.map(|time| {
                        TimeFormat::for_file().format_time(time.total_milliseconds() as u128, false)
                    }),
                })
                .collect();

            segments.push(Segment {
                name: segment.name().to_string(),
                segment_history,
                personal_best_split_time,
                best_segment_time,
            });
        }

        Self {
            game_name: run.game_name().to_string(),
            category_name: run.category_name().to_string(),
            attempt_count: run.attempt_count() as usize,
            attempt_history,
            segments,
        }
    }

    pub fn with_game_name(mut self, game_name: &str) -> Self {
        self.game_name = game_name.to_string();
        self
    }

    pub fn with_category_name(mut self, category_name: &str) -> Self {
        self.category_name = category_name.to_string();
        self
    }

    pub fn with_splits(mut self, splits: Vec<&str>) -> Self {
        self.segments = splits
            .iter()
            .map(|split| Segment {
                name: split.to_string(),
                ..Default::default()
            })
            .collect();
        self
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Attempt {
    pub id: i32,
    pub started: Option<String>,
    pub ended: Option<String>,
    pub time: Option<String>,
    pub pause_time: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct SplitTime {
    pub time: Option<String>,
    pub id: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct Segment {
    pub name: String,
    pub personal_best_split_time: Option<String>,
    pub best_segment_time: Option<String>,
    pub segment_history: Vec<SplitTime>,
}

pub fn read_json<T: DeserializeOwned>(path: &str) -> Result<T, Box<dyn Error>> {
    let mut file = File::open(path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    let result: T = serde_json::from_str(&content)?;

    Ok(result)
}

pub fn write_json<T: Serialize>(path: &str, data: T) -> Result<(), Box<dyn Error>> {
    let serialized = serde_json::to_string_pretty(&data)?;
    let mut file = File::create(path)?;
    file.write_all(serialized.as_bytes())?;

    Ok(())
}
