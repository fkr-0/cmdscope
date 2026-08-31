use crate::HistoryEntry;
use crate::config::{ColumnConfig, DateFormat};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColumnId {
    Date,
    Pwd,
    Exit,
    Duration,
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
            DateFormat::Iso8601 => format!(
                "{}Z",
                format_datetime(entry.timestamp, true).replace(' ', "T")
            ),
            DateFormat::Epoch => normalize_timestamp(entry.timestamp).to_string(),
        },
        ColumnId::Pwd => entry.cwd.clone(),
        ColumnId::Exit => format!("exit {}", entry.exit),
        ColumnId::Duration => format_duration(entry.duration),
    }
}
fn format_relative(timestamp: i64, now: i64, long: bool) -> String {
    let t = normalize_timestamp(timestamp);
    if t >= now {
        return "now".into();
    }
    let s = u64::try_from(now.saturating_sub(t)).unwrap_or(u64::MAX);
    if long {
        format_seconds_long(s)
    } else {
        format!("{} ago", format_seconds(s))
    }
}
fn format_seconds(seconds: u64) -> String {
    for (s, u) in [
        (31_557_600, "y"),
        (2_630_016, "mo"),
        (86_400, "d"),
        (3_600, "h"),
        (60, "m"),
    ] {
        if seconds >= s {
            return format!("{}{u}", seconds / s);
        }
    }
    format!("{seconds}s")
}
fn format_seconds_long(seconds: u64) -> String {
    for (s, u) in [
        (31_557_600, "years"),
        (2_630_016, "months"),
        (86_400, "days"),
        (3_600, "hours"),
        (60, "minutes"),
    ] {
        if seconds >= s {
            let n = seconds / s;
            return format!("{n} {u} ago");
        }
    }
    format!("{seconds} seconds ago")
}
pub fn normalize_timestamp(timestamp: i64) -> i64 {
    let abs = timestamp.unsigned_abs();
    if abs < 50_000_000_000 {
        timestamp
    } else if abs < 50_000_000_000_000 {
        timestamp / 1_000
    } else if abs < 50_000_000_000_000_000 {
        timestamp / 1_000_000
    } else {
        timestamp / 1_000_000_000
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
    format!("{y:04}-{m:02}-{d:02}")
}
fn format_datetime(timestamp: i64, seconds: bool) -> String {
    let t = normalize_timestamp(timestamp);
    let ds = t.rem_euclid(86_400);
    let h = ds / 3_600;
    let m = ds / 60 % 60;
    let s = ds % 60;
    let date = format_date(timestamp);
    if seconds {
        format!("{date} {h:02}:{m:02}:{s:02}")
    } else {
        format!("{date} {h:02}:{m:02}")
    }
}
fn format_duration(nanoseconds: i64) -> String {
    let n = u64::try_from(nanoseconds).unwrap_or(0);
    if n == 0 {
        "0s".into()
    } else if n >= 1_000_000_000 {
        format!("{}s", n / 1_000_000_000)
    } else if n >= 1_000_000 {
        format!("{}ms", n / 1_000_000)
    } else if n >= 1_000 {
        format!("{}us", n / 1_000)
    } else {
        format!("{n}ns")
    }
}
pub fn truncate_end(input: &str, width: usize) -> String {
    if UnicodeWidthStr::width(input) <= width {
        return input.to_owned();
    }
    if width == 0 {
        return String::new();
    }
    let mut out = String::new();
    let mut used = 0;
    for c in input.chars() {
        let w = UnicodeWidthChar::width(c).unwrap_or(0);
        if used + w + 1 > width {
            break;
        }
        out.push(c);
        used += w
    }
    out.push('…');
    out
}
pub fn truncate_start(input: &str, width: usize) -> String {
    if UnicodeWidthStr::width(input) <= width {
        return input.to_owned();
    }
    if width == 0 {
        return String::new();
    }
    let mut chars = Vec::new();
    let mut used = 0;
    for c in input.chars().rev() {
        let w = UnicodeWidthChar::width(c).unwrap_or(0);
        if used + w + 1 > width {
            break;
        }
        chars.push(c);
        used += w
    }
    chars.reverse();
    format!("…{}", chars.into_iter().collect::<String>())
}
