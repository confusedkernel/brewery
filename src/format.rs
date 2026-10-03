//! Pure formatting helpers shared by the app and UI layers.

pub fn format_size(size_kb: u64) -> String {
    let megabytes = size_kb as f64 / 1024.0;
    if megabytes < 1024.0 {
        return format!("{megabytes:.1}M");
    }
    let gigabytes = megabytes / 1024.0;
    format!("{gigabytes:.1}G")
}

/// `YYYY-MM-DD` in UTC for a Unix timestamp. Days are the only precision an
/// install date needs, so this avoids pulling in a date crate.
pub fn format_date(unix_secs: u64) -> String {
    let days = (unix_secs / 86_400) as i64;
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}")
}

/// Coarse "how long ago" for an install date: `today`, `3 days ago`,
/// `2 months ago`, `1 year ago`.
pub fn format_age(secs_ago: u64) -> String {
    const DAY: u64 = 86_400;
    let days = secs_ago / DAY;

    let (count, unit) = if days == 0 {
        return "today".to_string();
    } else if days < 14 {
        (days, "day")
    } else if days < 60 {
        (days / 7, "week")
    } else if days < 365 {
        (days / 30, "month")
    } else {
        (days / 365, "year")
    };

    let plural = if count == 1 { "" } else { "s" };
    format!("{count} {unit}{plural} ago")
}

/// Howard Hinnant's days-to-civil algorithm, valid for the proleptic
/// Gregorian calendar.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = if month <= 2 { year + 1 } else { year };
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_unix_timestamps_as_utc_dates() {
        assert_eq!(format_date(0), "1970-01-01");
        assert_eq!(format_date(951_782_400), "2000-02-29");
        assert_eq!(format_date(1_770_570_764), "2026-02-08");
    }

    #[test]
    fn describes_ages_in_the_coarsest_sensible_unit() {
        const DAY: u64 = 86_400;
        assert_eq!(format_age(0), "today");
        assert_eq!(format_age(DAY - 1), "today");
        assert_eq!(format_age(DAY), "1 day ago");
        assert_eq!(format_age(5 * DAY), "5 days ago");
        assert_eq!(format_age(21 * DAY), "3 weeks ago");
        assert_eq!(format_age(90 * DAY), "3 months ago");
        assert_eq!(format_age(400 * DAY), "1 year ago");
        assert_eq!(format_age(800 * DAY), "2 years ago");
    }

    #[test]
    fn formats_megabytes_below_a_gigabyte() {
        assert_eq!(format_size(1024), "1.0M");
        assert_eq!(format_size(0), "0.0M");
    }

    #[test]
    fn switches_to_gigabytes_at_the_boundary() {
        assert_eq!(format_size(1024 * 1024), "1.0G");
        assert_eq!(format_size(1024 * 1536), "1.5G");
    }
}
