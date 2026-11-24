use chrono::{Duration, NaiveDate};
use std::collections::{BTreeMap, BTreeSet};

use crate::format::local_datetime;
use crate::models::Recording;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Summary {
    pub total_count: u64,
    pub total_ms: i64,
    pub average_ms: i64,
    pub active_days: u64,
    pub today_count: u64,
    pub today_ms: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DayTotal {
    pub date: NaiveDate,
    pub count: u32,
    pub ms: i64,
}

pub fn day_of(recording: &Recording) -> NaiveDate {
    local_datetime(recording.created_at).date_naive()
}

/// Aggregates real recordings only, an empty list yields a zeroed summary.
pub fn summarize(recordings: &[Recording], today: NaiveDate) -> Summary {
    let mut summary = Summary::default();
    let mut active_days = BTreeSet::new();

    for recording in recordings {
        let day = day_of(recording);
        summary.total_count += 1;
        summary.total_ms += recording.duration_ms;
        active_days.insert(day);
        if day == today {
            summary.today_count += 1;
            summary.today_ms += recording.duration_ms;
        }
    }

    summary.active_days = active_days.len() as u64;
    if summary.total_count > 0 {
        summary.average_ms = summary.total_ms / summary.total_count as i64;
    }
    summary
}

/// Activity of the last seven days, oldest first, including empty days.
pub fn week_activity(recordings: &[Recording], today: NaiveDate) -> Vec<DayTotal> {
    let start = today - Duration::days(6);
    (0..7)
        .map(|offset| {
            let date = start + Duration::days(offset);
            let mut total = DayTotal {
                date,
                count: 0,
                ms: 0,
            };
            for recording in recordings {
                if day_of(recording) == date {
                    total.count += 1;
                    total.ms += recording.duration_ms;
                }
            }
            total
        })
        .collect()
}

/// Recordings grouped per day, newest day first, newest take first.
pub fn group_by_day(recordings: &[Recording]) -> Vec<(NaiveDate, Vec<Recording>)> {
    let mut groups: BTreeMap<NaiveDate, Vec<Recording>> = BTreeMap::new();
    for recording in recordings {
        groups.entry(day_of(recording)).or_default().push(recording.clone());
    }
    groups
        .into_iter()
        .rev()
        .map(|(day, mut items)| {
            items.sort_by(|a, b| b.created_at.cmp(&a.created_at));
            (day, items)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Local, TimeZone};

    fn at(day: u32, hour: u32, duration_ms: i64) -> Recording {
        let local = Local
            .with_ymd_and_hms(2026, 11, day, hour, 0, 0)
            .single()
            .expect("valid local time");
        Recording {
            id: format!("id-{day}-{hour}"),
            path: format!("/tmp/{day}-{hour}.wav"),
            duration_ms,
            created_at: local.timestamp_millis(),
        }
    }

    fn day(day: u32) -> NaiveDate {
        Local
            .with_ymd_and_hms(2026, 11, day, 12, 0, 0)
            .single()
            .expect("valid local time")
            .date_naive()
    }

    #[test]
    fn empty_history_is_zeroed() {
        let summary = summarize(&[], day(24));
        assert_eq!(summary, Summary::default());
    }

    #[test]
    fn summary_counts_today_and_active_days() {
        let items = vec![at(24, 9, 60_000), at(24, 18, 30_000), at(20, 8, 90_000)];
        let summary = summarize(&items, day(24));
        assert_eq!(summary.total_count, 3);
        assert_eq!(summary.total_ms, 180_000);
        assert_eq!(summary.average_ms, 60_000);
        assert_eq!(summary.active_days, 2);
        assert_eq!(summary.today_count, 2);
        assert_eq!(summary.today_ms, 90_000);
    }

    #[test]
    fn week_has_seven_days_oldest_first() {
        let items = vec![at(24, 9, 60_000), at(22, 9, 30_000)];
        let week = week_activity(&items, day(24));
        assert_eq!(week.len(), 7);
        assert_eq!(week[6].date, day(24));
        assert_eq!(week[6].count, 1);
        assert_eq!(week[4].date, day(22));
        assert_eq!(week[4].ms, 30_000);
        assert_eq!(week[0].count, 0);
    }

    #[test]
    fn grouping_sorts_days_and_takes() {
        let items = vec![at(22, 9, 10_000), at(24, 8, 10_000), at(24, 19, 10_000)];
        let groups = group_by_day(&items);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].0, day(24));
        assert_eq!(groups[0].1[0].created_at > groups[0].1[1].created_at, true);
        assert_eq!(groups[1].0, day(22));
    }
}
