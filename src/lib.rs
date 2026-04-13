use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CalendarEvent {
    pub title: String,
    pub date_iso: String,
    pub day_label: String,
    pub time_label: String,
    pub kind: String,
    pub attendees: u32,
    pub start_epoch: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CalendarPayload {
    pub timezone: String,
    pub updated: String,
    pub events: Vec<CalendarEvent>,
}

pub fn infer_type(title: &str) -> String {
    let lower = title.to_ascii_lowercase();
    for (kind, keywords) in [
        ("standup", &["standup", "stand-up", "stand up", "daily scrum", "daily sync"][..]),
        ("retro", &["retro", "retrospective"][..]),
        ("planning", &["planning", "sprint", "roadmap", "kickoff", "kick-off"][..]),
        ("interview", &["interview", "hiring", "panel", "candidate"][..]),
        ("presentation", &["demo", "presentation", "showcase", "all hands", "all-hands", "town hall"][..]),
        ("personal", &["1:1", "one-on-one", "one on one", "catch up", "catch-up"][..]),
        ("review", &["review", "rfc", "design review", "code review", "pr review"][..]),
    ] {
        if keywords.iter().any(|keyword| lower.contains(keyword)) {
            return kind.to_string();
        }
    }
    "review".to_string()
}

fn month_name(month: u8) -> &'static str {
    match month {
        1 => "Jan", 2 => "Feb", 3 => "Mar", 4 => "Apr", 5 => "May", 6 => "Jun",
        7 => "Jul", 8 => "Aug", 9 => "Sep", 10 => "Oct", 11 => "Nov", 12 => "Dec",
        _ => "???",
    }
}

fn weekday_abbrev(year: i32, month: u8, day: u8) -> &'static str {
    let days = days_from_civil(year, month, day);
    let weekday = ((days + 4) % 7 + 7) % 7;
    match weekday {
        0 => "Sun", 1 => "Mon", 2 => "Tue", 3 => "Wed",
        4 => "Thu", 5 => "Fri", 6 => "Sat",
        _ => "???",
    }
}

fn days_from_civil(year: i32, month: u8, day: u8) -> i64 {
    let year = i64::from(year) - i64::from(month <= 2);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let yoe = year - era * 400;
    let month = i64::from(month);
    let day = i64::from(day);
    let doy = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(days: i64) -> (i32, u8, u8) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    let year = year + if month <= 2 { 1 } else { 0 };
    (year as i32, month as u8, day as u8)
}

fn first_sunday(year: i32, month: u8) -> u8 {
    (1..=7).find(|day| weekday_abbrev(year, month, *day) == "Sun").unwrap_or(1)
}

fn timezone_offset_seconds(timezone: &str, year: i32, month: u8, day: u8, hour: u8) -> Result<i32, String> {
    match timezone {
        "UTC" => Ok(0),
        "Australia/Melbourne" => {
            let first_sunday_october = first_sunday(year, 10);
            let first_sunday_april = first_sunday(year, 4);
            let is_dst = if !(4..10).contains(&month) {
                true
            } else if (5..=9).contains(&month) {
                false
            } else if month == 10 {
                day > first_sunday_october || (day == first_sunday_october && hour >= 2)
            } else {
                day < first_sunday_april || (day == first_sunday_april && hour < 3)
            };
            Ok(if is_dst { 11 * 3_600 } else { 10 * 3_600 })
        }
        other => Err(format!("unsupported timezone '{other}'")),
    }
}

fn local_datetime_to_epoch(year: i32, month: u8, day: u8, hour: u8, minute: u8, second: u8, timezone: &str) -> Result<u64, String> {
    let offset = timezone_offset_seconds(timezone, year, month, day, hour)?;
    let local = days_from_civil(year, month, day) * 86_400
        + i64::from(hour) * 3_600 + i64::from(minute) * 60 + i64::from(second);
    Ok((local - i64::from(offset)) as u64)
}

pub fn epoch_from_utc(year: i32, month: u8, day: u8, hour: u8, minute: u8, second: u8) -> u64 {
    (days_from_civil(year, month, day) * 86_400
        + i64::from(hour) * 3_600 + i64::from(minute) * 60 + i64::from(second)) as u64
}

fn epoch_to_local_components(epoch_secs: u64, timezone: &str) -> (i32, u8, u8, u8, u8, u8) {
    let mut offset = if timezone == "Australia/Melbourne" { 10 * 3_600 } else { 0 };
    for _ in 0..2 {
        let shifted = (epoch_secs as i64 + i64::from(offset)) as u64;
        let (year, month, day, hour, minute, second) = utc_components(shifted);
        offset = timezone_offset_seconds(timezone, year, month, day, hour).unwrap_or(0);
        if timezone == "UTC" {
            return (year, month, day, hour, minute, second);
        }
    }
    utc_components((epoch_secs as i64 + i64::from(offset)) as u64)
}

fn utc_components(epoch_secs: u64) -> (i32, u8, u8, u8, u8, u8) {
    let days = (epoch_secs / 86_400) as i64;
    let seconds_today = epoch_secs % 86_400;
    let (year, month, day) = civil_from_days(days);
    (year, month, day, (seconds_today / 3_600) as u8, ((seconds_today / 60) % 60) as u8, (seconds_today % 60) as u8)
}

fn unfold_ics(input: &str) -> String {
    let mut output = String::new();
    let mut current = String::new();
    for raw_line in input.replace("\r\n", "\n").split('\n') {
        if raw_line.starts_with(' ') || raw_line.starts_with('\t') {
            current.push_str(raw_line.trim_start());
            continue;
        }
        if !current.is_empty() {
            output.push_str(&current);
            output.push('\n');
        }
        current = raw_line.to_string();
    }
    if !current.is_empty() {
        output.push_str(&current);
    }
    output
}

fn parse_date(value: &str) -> Result<(i32, u8, u8), String> {
    if value.len() < 8 {
        return Err(format!("unsupported DATE value '{value}'"));
    }
    let year = value[0..4].parse().map_err(|_| format!("bad year in '{value}'"))?;
    let month = value[4..6].parse().map_err(|_| format!("bad month in '{value}'"))?;
    let day = value[6..8].parse().map_err(|_| format!("bad day in '{value}'"))?;
    Ok((year, month, day))
}

fn parse_ics_datetime(value: &str, tzid: Option<&str>, value_is_date: bool, local_tz: &str) -> Result<u64, String> {
    if value_is_date || value.len() == 8 {
        let (year, month, day) = parse_date(value)?;
        return local_datetime_to_epoch(year, month, day, 0, 0, 0, tzid.unwrap_or(local_tz));
    }
    let is_utc = value.ends_with('Z');
    let core = value.trim_end_matches('Z');
    if core.len() < 15 {
        return Err(format!("unsupported DTSTART value '{value}'"));
    }
    let year = core[0..4].parse().map_err(|_| format!("bad year in '{value}'"))?;
    let month = core[4..6].parse().map_err(|_| format!("bad month in '{value}'"))?;
    let day = core[6..8].parse().map_err(|_| format!("bad day in '{value}'"))?;
    let hour = core[9..11].parse().map_err(|_| format!("bad hour in '{value}'"))?;
    let minute = core[11..13].parse().map_err(|_| format!("bad minute in '{value}'"))?;
    let second = core[13..15].parse().map_err(|_| format!("bad second in '{value}'"))?;
    if is_utc {
        Ok(epoch_from_utc(year, month, day, hour, minute, second))
    } else {
        local_datetime_to_epoch(year, month, day, hour, minute, second, tzid.unwrap_or(local_tz))
    }
}

#[derive(Default)]
struct EventBuilder {
    summary: Option<String>,
    dtstart: Option<(String, Option<String>, bool)>,
    attendees: u32,
}

impl EventBuilder {
    fn apply_line(&mut self, line: &str) {
        let Some((lhs, rhs)) = line.split_once(':') else { return; };
        let mut key_parts = lhs.split(';');
        let key = key_parts.next().unwrap_or_default();
        match key {
            "SUMMARY" => self.summary = Some(rhs.trim().to_string()),
            "DTSTART" => {
                let mut tzid = None;
                let mut value_is_date = false;
                for part in key_parts {
                    if let Some(value) = part.strip_prefix("TZID=") {
                        tzid = Some(value.to_string());
                    }
                    if part == "VALUE=DATE" {
                        value_is_date = true;
                    }
                }
                self.dtstart = Some((rhs.trim().to_string(), tzid, value_is_date));
            }
            "ATTENDEE" => self.attendees += 1,
            _ => {}
        }
    }

    fn finish(self, now_secs: u64, cutoff_secs: u64, local_tz: &str) -> Result<Option<CalendarEvent>, String> {
        let Some(summary) = self.summary else { return Ok(None); };
        let Some((raw, tzid, value_is_date)) = self.dtstart else { return Ok(None); };
        let start_epoch = parse_ics_datetime(&raw, tzid.as_deref(), value_is_date, local_tz)?;
        if !(now_secs..=cutoff_secs).contains(&start_epoch) {
            return Ok(None);
        }
        let (year, month, day, hour, minute, _) = epoch_to_local_components(start_epoch, local_tz);
        let kind = infer_type(&summary);
        Ok(Some(CalendarEvent {
            title: summary,
            date_iso: format!("{year:04}-{month:02}-{day:02}"),
            day_label: format!("{} {:02} {}", weekday_abbrev(year, month, day), day, month_name(month)),
            time_label: if value_is_date { "All day".to_string() } else { format!("{hour:02}:{minute:02}") },
            kind,
            attendees: self.attendees.max(1),
            start_epoch,
        }))
    }
}

pub fn parse_ics(ics_text: &str, now_secs: u64, days: u32, local_tz: &str) -> Result<Vec<CalendarEvent>, String> {
    let unfolded = unfold_ics(ics_text);
    let cutoff = now_secs + u64::from(days) * 86_400;
    let mut events = Vec::new();
    let mut current = EventBuilder::default();
    let mut in_event = false;
    for line in unfolded.lines() {
        match line {
            "BEGIN:VEVENT" => { in_event = true; current = EventBuilder::default(); }
            "END:VEVENT" => {
                if in_event {
                    if let Some(event) = std::mem::take(&mut current).finish(now_secs, cutoff, local_tz)? {
                        events.push(event);
                    }
                }
                in_event = false;
            }
            _ if in_event => current.apply_line(line),
            _ => {}
        }
    }
    events.sort_by_key(|event| event.start_epoch);
    Ok(events)
}

pub fn parse_calendar_payload(ics_text: &str, now_secs: u64, local_tz: &str) -> Result<CalendarPayload, String> {
    let events = parse_ics(ics_text, now_secs, 7, local_tz)?;
    let hh = (now_secs % 86400) / 3600;
    let mm = (now_secs % 3600) / 60;
    Ok(CalendarPayload {
        timezone: local_tz.to_string(),
        updated: format!("Updated {:02}:{:02}", hh, mm),
        events,
    })
}
