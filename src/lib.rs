//! open-scrantic: a reimplementation of Screen Antics: Johnny Castaway (Sierra/Dynamix, 1992)
//! that runs on the original, installed screensaver files.

pub mod assets;
pub mod audio;
pub mod engine;
pub mod present;

/// Directory of the running executable: where the screensaver files and `SCRANTIC.INI` live.
pub fn app_dir() -> std::path::PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(std::path::Path::to_path_buf))
        .unwrap_or_else(|| ".".into())
}
