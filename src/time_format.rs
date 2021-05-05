const MSEC_HOUR: u128 = 3600000;
const MSEC_MINUTE: u128 = 60000;
const MSEC_SECOND: u128 = 1000;

pub struct TimeFormat {
    pub hours: usize,
    pub minutes: usize,
    pub seconds: usize,
    pub msecs: usize,
    pub allow_shorten: bool,
    pub always_prefix: bool,
}

impl TimeFormat {
    pub fn for_diff() -> Self {
        let mut format = TimeFormat::default();
        format.always_prefix = true;
        format
    }

    pub fn for_file() -> Self {
        let mut format = TimeFormat::default();
        format.allow_shorten = false;
        format
    }

    pub fn format_time(
		&self,
        time: u128,
        negative: bool,
    ) -> String {
        let prefix = if negative {
            "-"
        } else if self.always_prefix {
            "+"
        } else {
            ""
        };
        let mut time = time;
        let hours = time / MSEC_HOUR;
        time -= hours * MSEC_HOUR;
        let minutes = time / MSEC_MINUTE;
        time -= minutes * MSEC_MINUTE;
        let seconds = time / MSEC_SECOND;
        time -= seconds * MSEC_SECOND;

        if self.allow_shorten && hours == 0 {
            if minutes == 0 {
                return format!(
                    "{}{}.{}",
                    prefix,
                    pad_zeroes(seconds, self.seconds),
                    pad_zeroes(time, self.msecs),
                );
            }
            return format!(
                "{}{}:{}.{}",
                prefix,
                pad_zeroes(minutes, self.minutes),
                pad_zeroes(seconds, self.seconds),
                pad_zeroes(time, self.msecs),
            );
        }
        format!(
            "{}{}:{}:{}.{}",
            prefix,
            pad_zeroes(hours, self.hours),
            pad_zeroes(minutes, self.minutes),
            pad_zeroes(seconds, self.seconds),
            pad_zeroes(time, self.msecs),
        )
    }
}

impl Default for TimeFormat {
    fn default() -> Self {
        Self {
            hours: 2,
            minutes: 2,
            seconds: 2,
            msecs: 3,
            allow_shorten: true,
            always_prefix: false,
        }
    }
}

fn pad_zeroes(time: u128, length: usize) -> String {
    let str_length = time.to_string().chars().count();
    if str_length >= length {
        return format!("{}", time);
    }
    let count = length - str_length;
    let zeroes = "0".repeat(count);
    format!("{}{}", zeroes, time)
}