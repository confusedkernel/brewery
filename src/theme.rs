use ratatui::style::Color;

// Theme struct to hold all colors
#[derive(Clone, Copy)]
pub struct Theme {
    // Primary accents
    pub amber: Color,
    pub copper: Color,
    pub dark_amber: Color,

    // Text colors
    pub hop_green: Color,
    pub text_primary: Color,
    pub text_secondary: Color,

    // Backgrounds
    pub bg_main: Color,
    pub bg_panel: Color,
    pub bg_header: Color,

    // Accents
    pub text_on_accent: Color,
    pub border: Color,
}

impl Theme {
    pub fn light() -> Self {
        Self {
            // Primary accents - same for both
            amber: Color::Rgb(212, 145, 40),
            copper: Color::Rgb(166, 100, 50),
            dark_amber: Color::Rgb(140, 90, 45),

            // Text - dark on light
            hop_green: Color::Rgb(76, 132, 60),
            text_primary: Color::Rgb(70, 50, 35),
            text_secondary: Color::Rgb(120, 90, 60),

            // Light backgrounds
            bg_main: Color::Rgb(255, 250, 240),
            bg_panel: Color::Rgb(250, 240, 220),
            bg_header: Color::Rgb(255, 248, 230),

            // Accents
            text_on_accent: Color::Rgb(255, 255, 255),
            border: Color::Rgb(180, 150, 120),
        }
    }

    pub fn dark() -> Self {
        Self {
            // Primary accents - slightly brighter for dark mode
            amber: Color::Rgb(255, 191, 0),
            copper: Color::Rgb(205, 133, 63),
            dark_amber: Color::Rgb(184, 134, 11),

            // Text - light on dark
            hop_green: Color::Rgb(124, 179, 66),
            text_primary: Color::Rgb(245, 235, 220),
            text_secondary: Color::Rgb(200, 180, 160),

            // Dark backgrounds
            bg_main: Color::Rgb(30, 22, 16),
            bg_panel: Color::Rgb(45, 32, 22),
            bg_header: Color::Rgb(38, 28, 20),

            // Accents
            text_on_accent: Color::Rgb(25, 18, 12),
            border: Color::Rgb(100, 75, 55),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ThemeMode {
    Light,
    Dark,
    Auto,
}

pub fn detect_system_theme() -> Theme {
    // Check COLORFGBG env var (set by many terminals)
    // Format: "fg;bg" where bg > 7 typically means dark background
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
        // Many modern terminals default to dark
        if term_lower.contains("iterm")
            || term_lower.contains("alacritty")
            || term_lower.contains("kitty")
        {
            // Check if there's a theme preference
            if let Ok(appearance) = std::env::var("TERM_PROGRAM_VERSION") {
                if appearance.contains("light") {
                    return Theme::light();
                }
            }
        }
    }

    // macOS: check AppleInterfaceStyle
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = std::process::Command::new("defaults")
            .args(["read", "-g", "AppleInterfaceStyle"])
            .output()
        {
            if output.status.success() {
                let style = String::from_utf8_lossy(&output.stdout);
                if style.trim().eq_ignore_ascii_case("dark") {
                    return Theme::dark();
                }
            }
            // If command succeeds but no "Dark", it's light mode
            // If command fails, key doesn't exist = light mode
            return Theme::light();
        }
    }

    // Default to dark (more common in terminals)
    Theme::dark()
}
