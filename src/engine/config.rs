//! Persistent settings and story progress, stored like the original SCRANTIC.INI
//! (section [ScreenSaver.ScreenAntics], same keys and defaults).

use std::fs;
use std::path::PathBuf;

const SECTION: &str = "[ScreenSaver.ScreenAntics]";

#[derive(Clone, Debug)]
pub struct Config {
    pub background: bool,
    pub clouds: bool,
    pub waves: bool,
    pub sounds: bool,
    /// Furthest story chapter unlocked ("NumDays").
    pub num_days: i32,
    /// Next story chapter to show ("Introduction").
    pub intro: i32,
    pub intro_saved: i32,
    pub year: i32,
    pub month: i32,
    pub day: i32,
    /// Start of the day/night cycle, as HHMM ("StartTime").
    pub start_time: i32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            background: true,
            clouds: true,
            waves: true,
            sounds: true,
            num_days: 0,
            intro: 0,
            intro_saved: 0,
            year: 0,
            month: 0,
            day: 0,
            start_time: 900,
        }
    }
}

impl Config {
    pub fn path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("open-scrantic").join("SCRANTIC.INI"))
    }

    pub fn load() -> Self {
        let mut cfg = Self::default();
        let Some(text) = Self::path().and_then(|p| fs::read_to_string(p).ok()) else {
            return cfg;
        };
        let mut in_section = false;
        for line in text.lines().map(str::trim) {
            if line.starts_with('[') {
                in_section = line.eq_ignore_ascii_case(SECTION);
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            if !in_section {
                continue;
            }
            let Ok(v) = value.trim().parse::<i32>() else { continue };
            match key.trim().to_ascii_lowercase().as_str() {
                "background" => cfg.background = v != 0,
                "clouds" => cfg.clouds = v != 0,
                "waves" => cfg.waves = v != 0,
                "sounds" => cfg.sounds = v != 0,
                "numdays" => cfg.num_days = v,
                "introduction" => cfg.intro = v,
                "currentyear" => cfg.year = v,
                "currentmonth" => cfg.month = v,
                "currentday" => cfg.day = v,
                "starttime" => cfg.start_time = v,
                _ => {}
            }
        }
        cfg
    }

    pub fn save(&self) -> std::io::Result<()> {
        let Some(path) = Self::path() else { return Ok(()) };
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let intro = if self.intro_saved != 0 {
            self.intro_saved
        } else {
            self.intro
        };
        let b = |v: bool| i32::from(v);
        let text = format!(
            "{SECTION}\nBackground={}\nClouds={}\nWaves={}\nSounds={}\nNumDays={}\nIntroduction={}\nCurrentYear={}\nCurrentMonth={}\nCurrentDay={}\nStartTime={}\n",
            b(self.background),
            b(self.clouds),
            b(self.waves),
            b(self.sounds),
            self.num_days,
            intro,
            self.year,
            self.month,
            self.day,
            self.start_time
        );
        fs::write(path, text)
    }
}
