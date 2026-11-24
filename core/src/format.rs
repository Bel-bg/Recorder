use chrono::{DateTime, Local};

/// Timer style clock used for the recorder, `mm:ss` or `h:mm:ss`.
pub fn clock(ms: i64) -> String {
    let ms = ms.max(0);
    let total_seconds = ms / 1000;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes:02}:{seconds:02}")
    }
}

/// Short duration label such as `48 s` or `2 min 05 s`.
pub fn duration(ms: i64) -> String {
    let ms = ms.max(0);
    let total_seconds = (ms + 500) / 1000;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    if minutes == 0 {
        format!("{seconds} s")
    } else {
        format!("{minutes} min {seconds:02} s")
    }
}

/// Accumulated practice time, split in hours and minutes.
pub fn practice_time(ms: i64) -> String {
    let ms = ms.max(0);
    let total_minutes = ms / 60_000;
    if total_minutes < 1 {
        return format!("{} s", ms / 1000);
    }
    let hours = total_minutes / 60;
    let minutes = total_minutes % 60;
    if hours == 0 {
        format!("{minutes} min")
    } else {
        format!("{hours} h {minutes:02}")
    }
}

pub fn local_datetime(ts_ms: i64) -> DateTime<Local> {
    DateTime::from_timestamp_millis(ts_ms)
        .map(|utc| utc.with_timezone(&Local))
        .unwrap_or_else(Local::now)
}

/// `14:32` in the device timezone.
pub fn time_of_day(ts_ms: i64) -> String {
    local_datetime(ts_ms).format("%H:%M").to_string()
}

/// `Aujourd'hui`, `Hier` or a written date.
pub fn day_label(day: chrono::NaiveDate, today: chrono::NaiveDate) -> String {
    let delta = (today - day).num_days();
    match delta {
        0 => "Aujourd'hui".to_string(),
        1 => "Hier".to_string(),
        _ => day.format("%d/%m/%Y").to_string(),
    }
}

/// Single letter label for the weekly chart.
pub fn weekday_initial(day: chrono::NaiveDate) -> &'static str {
    use chrono::Datelike;
    const LETTERS: [&str; 7] = ["L", "M", "M", "J", "V", "S", "D"];
    LETTERS[day.weekday().num_days_from_monday() as usize]
}

pub fn today() -> chrono::NaiveDate {
    Local::now().date_naive()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_uses_minutes_and_hours() {
        assert_eq!(clock(0), "00:00");
        assert_eq!(clock(7_400), "00:07");
        assert_eq!(clock(3_723_000), "1:02:03");
    }

    #[test]
    fn durations_are_human_readable() {
        assert_eq!(duration(4_800), "5 s");
        assert_eq!(duration(125_000), "2 min 05 s");
        assert_eq!(practice_time(90_000), "1 min");
        assert_eq!(practice_time(3_720_000), "1 h 02");
    }
}
