//! Loading of the original, installed screensaver files: SCRANTIC.SCR, RESOURCE.MAP and RESOURCE.001.

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
            Self::Missing(name) => write!(f, "missing screensaver file: {name}"),
            Self::Format(msg) => write!(f, "invalid screensaver data: {msg}"),
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

pub struct ScreensaverData {
    pub archive: Archive,
    pub exe: ExeData,
}

/// Reads a file from `dir`, matching the name case-insensitively (installs are often upper case).
fn read_ci(dir: &Path, name: &str) -> Result<Vec<u8>> {
    let entries =
        fs::read_dir(dir).map_err(|e| AssetError::Missing(format!("screensaver directory {} ({e})", dir.display())))?;
    for entry in entries.flatten() {
        if entry.file_name().to_string_lossy().eq_ignore_ascii_case(name) {
            return Ok(fs::read(entry.path())?);
        }
    }
    Err(AssetError::Missing(format!("{} in {}", name, dir.display())))
}

impl ScreensaverData {
    pub fn load(dir: &Path) -> Result<Self> {
        let exe = read_ci(dir, "SCRANTIC.SCR")?;
        let map = read_ci(dir, "RESOURCE.MAP")?;
        let volume = read_ci(dir, "RESOURCE.001")?;
        if volume.len() < 1024 {
            return Err(AssetError::Format(
                "RESOURCE.001 is the installer placeholder; install the screensaver first".into(),
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
        let data = ScreensaverData::load(&dir).expect("load");
        assert_eq!(data.exe.scenes.len(), 80);
        assert_eq!(data.exe.sounds.len(), 23);
        assert_eq!(data.exe.paths.len(), 35);
        assert_eq!(data.archive.bmp("JOHNWALK.BMP").unwrap().sprites.len(), 36);
        assert_eq!(data.archive.ttm("FISHWALK.TTM").unwrap().scenes, vec![(19, 1)]);
        let ocean = data.archive.scr("OCEAN00.SCR").unwrap();
        assert_eq!((ocean.w, ocean.h), (640, 480));
    }

    #[test]
    fn corrupt_input_is_an_error_not_a_panic() {
        assert!(exe::ExeData::parse(&[]).is_err());
        assert!(exe::ExeData::parse(&[0; 100]).is_err());
        let mut mz = vec![0u8; 0x80];
        mz[..2].copy_from_slice(b"MZ");
        mz[0x3C] = 0xF0;
        assert!(exe::ExeData::parse(&mz).is_err());

        // A map announcing 65535 entries over a tiny volume.
        let mut map = vec![0u8; 21];
        map[19] = 0xFF;
        map[20] = 0xFF;
        assert!(resource::Archive::new(&map, &[0; 32]).is_err());

        assert!(resource::decompress(&[2, 0xFF, 0xFF, 0xFF, 0x7F]).is_err()); // absurd size
        assert!(resource::decompress(&[2, 8, 0, 0, 0, 0xFF, 0xFF, 0xFF, 0xFF]).is_err()); // invalid LZW code
        assert!(resource::decompress(&[9, 0, 0, 0, 0]).is_err()); // unknown method
    }

    #[test]
    fn truncated_installs_are_errors_not_panics() {
        let Some(dir) = data_dir() else { return };
        let exe = fs::read(dir.join("SCRANTIC.SCR")).expect("SCRANTIC.SCR");
        let map = fs::read(dir.join("RESOURCE.MAP")).expect("RESOURCE.MAP");
        let volume = fs::read(dir.join("RESOURCE.001")).expect("RESOURCE.001");
        for cut in (0..exe.len()).step_by(997) {
            let _ = exe::ExeData::parse(&exe[..cut]);
        }
        for cut in (0..volume.len()).step_by(4999) {
            let _ = resource::Archive::new(&map, &volume[..cut]);
        }
        for cut in 0..map.len() {
            let _ = resource::Archive::new(&map[..cut], &volume);
        }
    }
}
