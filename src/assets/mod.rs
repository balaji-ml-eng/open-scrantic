//! Loading of the original, installed game files: SCRANTIC.SCR, RESOURCE.MAP and RESOURCE.001.

pub mod exe;
pub mod resource;

use std::fmt;
use std::fs;
use std::path::Path;

pub use exe::ExeData;
pub use resource::Archive;

#[derive(Debug)]
pub enum AssetError {
    Io(std::io::Error),
    Missing(String),
    Format(String),
}

impl fmt::Display for AssetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::Missing(name) => write!(f, "missing game file: {name}"),
            Self::Format(msg) => write!(f, "invalid game data: {msg}"),
        }
    }
}

impl std::error::Error for AssetError {}

impl From<std::io::Error> for AssetError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

pub type Result<T> = std::result::Result<T, AssetError>;

pub struct GameData {
    pub archive: Archive,
    pub exe: ExeData,
}

/// Reads a file from `dir`, matching the name case-insensitively (installs are often upper case).
fn read_ci(dir: &Path, name: &str) -> Result<Vec<u8>> {
    let entries =
        fs::read_dir(dir).map_err(|e| AssetError::Missing(format!("game directory {} ({e})", dir.display())))?;
    for entry in entries.flatten() {
        if entry.file_name().to_string_lossy().eq_ignore_ascii_case(name) {
            return Ok(fs::read(entry.path())?);
        }
    }
    Err(AssetError::Missing(format!("{} in {}", name, dir.display())))
}

impl GameData {
    pub fn load(dir: &Path) -> Result<Self> {
        let exe = read_ci(dir, "SCRANTIC.SCR")?;
        let map = read_ci(dir, "RESOURCE.MAP")?;
        let volume = read_ci(dir, "RESOURCE.001")?;
        if volume.len() < 1024 {
            return Err(AssetError::Format(
                "RESOURCE.001 is the installer placeholder; install the game first".into(),
            ));
        }
        Ok(Self {
            archive: Archive::new(&map, &volume)?,
            exe: ExeData::parse(&exe)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Set OPEN_SCRANTIC_DATA to an install directory to run data-dependent tests.
    fn data_dir() -> Option<std::path::PathBuf> {
        std::env::var_os("OPEN_SCRANTIC_DATA").map(Into::into)
    }

    #[test]
    fn loads_installed_game() {
        let Some(dir) = data_dir() else { return };
        let data = GameData::load(&dir).expect("load");
        assert_eq!(data.exe.scenes.len(), 80);
        assert_eq!(data.exe.sounds.len(), 23);
        assert_eq!(data.exe.paths.len(), 35);
        assert_eq!(data.archive.bmp("JOHNWALK.BMP").unwrap().sprites.len(), 36);
        assert_eq!(data.archive.ttm("FISHWALK.TTM").unwrap().scenes, vec![(19, 1)]);
        let ocean = data.archive.scr("OCEAN00.SCR").unwrap();
        assert_eq!((ocean.w, ocean.h), (640, 480));
    }
}
