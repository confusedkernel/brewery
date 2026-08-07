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
