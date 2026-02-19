use crate::app::App;

pub fn icon_label(app: &App, nerd: &str, ascii: &str) -> String {
    if app.icons_ascii {
        ascii.to_string()
    } else {
        nerd.to_string()
    }
}

pub fn symbol<'a>(app: &App, nerd: &'a str, ascii: &'a str) -> &'a str {
    if app.icons_ascii { ascii } else { nerd }
}

pub fn format_size(size_kb: u64) -> String {
    let megabytes = size_kb as f64 / 1024.0;
    if megabytes < 1024.0 {
        return format!("{megabytes:.1}M");
    }
    let gigabytes = megabytes / 1024.0;
    format!("{gigabytes:.1}G")
}
