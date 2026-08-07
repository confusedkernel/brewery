//! Pure formatting helpers shared by the app and UI layers.

pub fn format_size(size_kb: u64) -> String {
    let megabytes = size_kb as f64 / 1024.0;
    if megabytes < 1024.0 {
        return format!("{megabytes:.1}M");
    }
    let gigabytes = megabytes / 1024.0;
    format!("{gigabytes:.1}G")
}

#[cfg(test)]
mod tests {
    use super::*;

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
