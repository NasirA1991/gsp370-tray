use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// A status file older than this is ignored so the widget can fall back to
/// querying the dongle directly after the tray has exited.
pub const MAX_AGE: Duration = Duration::from_secs(90);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Status {
    pub state: String,
    pub battery: Option<u8>,
    pub device: Option<String>,
}

impl Status {
    pub fn print(&self) {
        println!("state={}", self.state);
        if let Some(battery) = self.battery {
            println!("battery={battery}");
        }
        if let Some(device) = &self.device {
            println!("device={device}");
        }
    }
}

pub fn state_dir() -> PathBuf {
    if let Ok(dir) = env::var("GSP370_STATE_DIR") {
        if !dir.is_empty() {
            return PathBuf::from(dir);
        }
    }
    if let Ok(dir) = env::var("XDG_RUNTIME_DIR") {
        if !dir.is_empty() {
            return PathBuf::from(dir).join("gsp370-tray");
        }
    }
    if let Ok(home) = env::var("HOME") {
        if !home.is_empty() {
            return PathBuf::from(home).join(".cache/gsp370-tray");
        }
    }
    env::temp_dir().join(format!("gsp370-tray-{}", unsafe { libc::geteuid() }))
}

pub fn ensure_dir() -> io::Result<PathBuf> {
    let dir = state_dir();
    fs::create_dir_all(&dir)?;
    let metadata = fs::symlink_metadata(&dir)?;
    if !metadata.file_type().is_dir() || metadata.file_type().is_symlink() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{} is not a private directory", dir.display()),
        ));
    }
    if metadata.permissions().mode() & 0o077 != 0 {
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o700))?;
    }
    Ok(dir)
}

pub fn lock_path() -> PathBuf {
    state_dir().join("tray.lock")
}

pub fn status_path() -> PathBuf {
    state_dir().join("status")
}

pub fn write(state: &str, battery: Option<u8>, device: Option<&str>) -> io::Result<()> {
    let path = ensure_dir()?.join("status");
    let mut body = format!("version=1\nstate={state}\n");
    if let Some(battery) = battery {
        body.push_str(&format!("battery={battery}\n"));
    }
    if let Some(device) = device.filter(|value| !value.is_empty()) {
        body.push_str(&format!("device={device}\n"));
    }
    atomic_write(&path, body.as_bytes())
}

pub fn read() -> Option<Status> {
    let path = status_path();
    let metadata = fs::metadata(&path).ok()?;
    let modified = metadata.modified().ok()?;
    let age = SystemTime::now().duration_since(modified).ok()?;
    if age > MAX_AGE {
        return None;
    }
    parse(&fs::read_to_string(path).ok()?)
}

pub fn parse(text: &str) -> Option<Status> {
    let mut state = None;
    let mut battery = None;
    let mut device = None;
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key.trim() {
            "state" => state = Some(value.trim().to_owned()),
            "battery" => battery = value.trim().parse::<u8>().ok(),
            "device" => {
                let value = value.trim();
                if !value.is_empty() {
                    device = Some(value.to_owned());
                }
            }
            _ => {}
        }
    }
    let state = state?;
    if !matches!(
        state.as_str(),
        "on" | "off" | "no-dongle" | "permission" | "error" | "unknown"
    ) {
        return None;
    }
    if battery.is_some_and(|level| level > 100) {
        return None;
    }
    Some(Status {
        state,
        battery,
        device,
    })
}

/// Write through a private temporary file and rename it into place, so a
/// crash can never leave a truncated config or status file.
pub fn atomic_write(path: &Path, data: &[u8]) -> io::Result<()> {
    let parent = path.parent().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "path has no parent directory")
    })?;
    fs::create_dir_all(parent)?;
    let name = path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no filename"))?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temp = parent.join(format!(
        ".{}.tmp.{}.{}",
        name.to_string_lossy(),
        std::process::id(),
        stamp
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(&temp)?;
    let result = (|| {
        file.write_all(data)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temp, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_and_minimal_status() {
        let status = parse("version=1\nstate=on\nbattery=83\ndevice=/dev/ttyACM0\n").unwrap();
        assert_eq!(status.state, "on");
        assert_eq!(status.battery, Some(83));
        assert_eq!(status.device.as_deref(), Some("/dev/ttyACM0"));

        let off = parse("state=off\n").unwrap();
        assert_eq!(off.state, "off");
        assert_eq!(off.battery, None);
        assert_eq!(off.device, None);
    }

    #[test]
    fn rejects_bad_states_and_batteries() {
        assert!(parse("state=on\nbattery=101\n").is_none());
        assert!(parse("state=banana\n").is_none());
        assert!(parse("battery=50\n").is_none());
    }

    #[test]
    fn atomic_write_replaces_without_partial_content() {
        let dir = env::temp_dir().join(format!("gsp370-status-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("status");
        atomic_write(&path, b"state=on\nbattery=83\n").unwrap();
        atomic_write(&path, b"state=off\n").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "state=off\n");
        let _ = fs::remove_dir_all(&dir);
    }
}
