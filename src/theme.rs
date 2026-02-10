use ratatui::style::Color;

#[derive(Clone, Copy)]
pub struct Theme {
    // Primary
    pub accent: Color,
    pub accent_secondary: Color,
    pub green: Color,
    pub yellow: Color,
    pub red: Color,
    pub orange: Color,

    // Text colors
    pub text_primary: Color,
    pub text_secondary: Color,
    pub text_muted: Color,

    // Backgrounds
    pub bg_main: Color,
    pub bg_panel: Color,
    pub bg_selection: Color,
    pub bg_dim: Color,

    // UI elements
    pub border: Color,
    pub border_active: Color,
    pub text_on_accent: Color,
}

impl Theme {
    pub fn light() -> Self {
        Self {
            accent: Color::Rgb(180, 130, 60),
            accent_secondary: Color::Rgb(150, 100, 65),
            green: Color::Rgb(100, 140, 80),
            yellow: Color::Rgb(190, 160, 100),
            red: Color::Rgb(180, 100, 90),
            orange: Color::Rgb(175, 125, 80),

            text_primary: Color::Rgb(60, 50, 40),
            text_secondary: Color::Rgb(110, 95, 80),
            text_muted: Color::Rgb(150, 135, 120),

            bg_main: Color::Rgb(252, 249, 242),
            bg_panel: Color::Rgb(248, 244, 235),
            bg_selection: Color::Rgb(230, 220, 200),
            bg_dim: Color::Rgb(180, 175, 165),

            border: Color::Rgb(200, 185, 165),
            border_active: Color::Rgb(180, 130, 60),
            text_on_accent: Color::Rgb(255, 252, 245),
        }
    }

    pub fn dark() -> Self {
        Self {
            accent: Color::Rgb(200, 155, 80),
            accent_secondary: Color::Rgb(175, 120, 75),
            green: Color::Rgb(130, 165, 95),
            yellow: Color::Rgb(195, 165, 105),
            red: Color::Rgb(185, 110, 100),
            orange: Color::Rgb(180, 130, 85),

            text_primary: Color::Rgb(210, 200, 185),
            text_secondary: Color::Rgb(145, 135, 120),
            text_muted: Color::Rgb(105, 95, 85),

            bg_main: Color::Rgb(30, 26, 22),
            bg_panel: Color::Rgb(38, 33, 28),
            bg_selection: Color::Rgb(60, 52, 45),
            bg_dim: Color::Rgb(18, 15, 12),

            border: Color::Rgb(65, 58, 50),
            border_active: Color::Rgb(200, 155, 80),
            text_on_accent: Color::Rgb(30, 26, 22),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ThemeMode {
    Light,
    Dark,
    Auto,
}

#[cfg(target_os = "macos")]
pub fn detect_system_theme() -> Theme {
    if let Ok(output) = std::process::Command::new("defaults")
        .args(["read", "-g", "AppleInterfaceStyle"])
        .output()
        && output.status.success()
    {
        let style = String::from_utf8_lossy(&output.stdout);
        if style.trim().eq_ignore_ascii_case("dark") {
            return Theme::dark();
        }
    }
    Theme::light()
}

#[cfg(not(target_os = "macos"))]
pub fn detect_system_theme() -> Theme {
    // Check COLORFGBG env var
    if let Ok(colorfgbg) = std::env::var("COLORFGBG") {
        if let Some(bg) = colorfgbg.split(';').last() {
            if let Ok(bg_num) = bg.parse::<u8>() {
                if bg_num == 0 || (bg_num >= 8 && bg_num <= 15) {
                    return Theme::dark();
                }
                return Theme::light();
            }
        }
    }

    // Check for common dark mode indicators
    if let Ok(term) = std::env::var("TERM_PROGRAM") {
        let term_lower = term.to_lowercase();
        if term_lower.contains("iterm")
            || term_lower.contains("alacritty")
            || term_lower.contains("kitty")
        {
            if let Ok(appearance) = std::env::var("TERM_PROGRAM_VERSION") {
                if appearance.contains("light") {
                    return Theme::light();
                }
            }
        }
    }

    Theme::light()
}
