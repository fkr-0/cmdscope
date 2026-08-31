use crate::{
    HistoryEntry,
    config::{ColumnConfig, DateFormat},
};
use std::fmt::Write;
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColumnId {
    Date,
    Pwd,
    Exit,
    Duration,
}

impl ColumnId {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "date" | "age" => Some(Self::Date),
            "pwd" => Some(Self::Pwd),
            "exit" => Some(Self::Exit),
            "duration" | "time" => Some(Self::Duration),
            _ => None,
        }
    }
}

pub fn format_column(
    entry: &HistoryEntry,
    column: ColumnId,
    config: &ColumnConfig,
    now: i64,
) -> String {
    match column {
        ColumnId::Date => match config.date_format {
            DateFormat::Relative => format_relative(entry.timestamp, now, false),
            DateFormat::RelativeLong => format_relative(entry.timestamp, now, true),
            DateFormat::Date => format_date(entry.timestamp),
            DateFormat::Datetime => format_datetime(entry.timestamp, false),
            DateFormat::DatetimeSeconds => format_datetime(entry.timestamp, true),
            DateFormat::Iso8601 => format_datetime(entry.timestamp, true).replace(' ', "T") + "Z",
            DateFormat::Epoch => normalize_timestamp(entry.timestamp).to_string(),
        },
        ColumnId::Pwd => entry.cwd.clone(),
        ColumnId::Exit => entry.exit.to_string(),
        ColumnId::Duration => format_duration(entry.duration),
    }
}

pub fn truncate_to(input: &str, width: usize) -> String {
    if UnicodeWidthStr::width(input) <= width {
        return input.to_string();
    }
    if width == 0 {
        return String::new();
    }
    let mut out = String::new();
    let mut used = 0;
    for c in input.chars() {
        let w = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
        if used + w + 1 > width {
            break;
        }
        out.push(c);
        used += w;
    }
    out.push('…');
    out
}

fn format_relative(timestamp: i64, now: i64, long: bool) -> String {
    let age = now.saturating_sub(normalize_timestamp(timestamp));
    if age <= 0 {
        return if long {
            "just now".into()
        } else {
            "now".into()
        };
    }
    let (value, unit) = if age >= 31_557_600 {
        (age / 31_557_600, "year")
    } else if age >= 2_630_016 {
        (age / 2_630_016, "month")
    } else if age >= 86_400 {
        (age / 86_400, "day")
    } else if age >= 3_600 {
        (age / 3_600, "hour")
    } else if age >= 60 {
        (age / 60, "minute")
    } else {
        (age, "second")
    };
    if long {
        format!("{value} {unit}{} ago", if value == 1 { "" } else { "s" })
    } else {
        format!(
            "{value}{} ago",
            match unit {
                "year" => "y",
                "month" => "mo",
                "day" => "d",
                "hour" => "h",
                "minute" => "m",
                _ => "s",
            }
        )
    }
}

fn format_duration(ns: i64) -> String {
    let ns = u64::try_from(ns).unwrap_or_default();
    if ns >= 1_000_000_000 {
        return format!("{}s", ns / 1_000_000_000);
    }
    if ns >= 1_000_000 {
        return format!("{}ms", ns / 1_000_000);
    }
    if ns >= 1_000 {
        return format!("{}us", ns / 1_000);
    }
    format!("{ns}ns")
}

fn normalize_timestamp(timestamp: i64) -> i64 {
    let magnitude = timestamp.unsigned_abs();
    if magnitude >= 100_000_000_000_000_000 {
        timestamp / 1_000_000_000
    } else if magnitude >= 100_000_000_000_000 {
        timestamp / 1_000_000
    } else if magnitude >= 100_000_000_000 {
        timestamp / 1_000
    } else {
        timestamp
    }
}

fn format_datetime(timestamp: i64, seconds: bool) -> String {
    let seconds_since_epoch = normalize_timestamp(timestamp);
    let day_seconds = seconds_since_epoch.rem_euclid(86_400);
    let hour = day_seconds / 3_600;
    let minute = day_seconds / 60 % 60;
    let second = day_seconds % 60;
    let date = format_date(timestamp);
    if seconds {
        format!("{date} {hour:02}:{minute:02}:{second:02}")
    } else {
        format!("{date} {hour:02}:{minute:02}")
    }
}

fn format_date(timestamp: i64) -> String {
    let days = normalize_timestamp(timestamp).div_euclid(86_400);
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    let y = y + if m <= 2 { 1 } else { 0 };
    let mut s = String::new();
    let _ = write!(&mut s, "{y:04}-{m:02}-{d:02}");
    s
}
