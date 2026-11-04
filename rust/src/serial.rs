use crate::protocol::*;

use std::ffi::CString;
use std::fs::{self, File};
use std::io;
use std::os::fd::{AsRawFd, FromRawFd};
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    NoDevice,
    PermissionDenied,
    OpenFailed,
    ConfigFailed,
    WriteFailed,
    ReadFailed,
    Timeout,
    HeadsetOff,
    Protocol,
}

#[derive(Debug, Clone)]
pub struct Error {
    pub code: ErrorCode,
    pub message: String,
}

impl Error {
    fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self { code, message: message.into() }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

pub struct Device {
    file: File,
    path: String,
}

fn read_attr(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().to_owned())
}

fn find_port() -> Option<String> {
    for entry in fs::read_dir("/sys/class/tty").ok()? {
        let entry = entry.ok()?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.starts_with("ttyACM") {
            continue;
        }
        let Ok(interface) = fs::canonicalize(entry.path().join("device")) else {
            continue;
        };
        let Some(usb_device) = interface.parent() else {
            continue;
        };
        if read_attr(&usb_device.join("idVendor")).as_deref() == Some("1395")
            && read_attr(&usb_device.join("idProduct")).as_deref() == Some("009a")
        {
            return Some(format!("/dev/{name}"));
        }
    }
    None
}

fn open_path(path: &str) -> Result<Device, Error> {
    let c_path = CString::new(path).map_err(|_| Error::new(ErrorCode::OpenFailed, "invalid path"))?;
    let fd = unsafe {
        libc::open(
            c_path.as_ptr(),
            libc::O_RDWR | libc::O_NOCTTY | libc::O_CLOEXEC | libc::O_NONBLOCK,
        )
    };
    if fd < 0 {
        let err = io::Error::last_os_error();
        let code = match err.raw_os_error() {
            Some(libc::EACCES) | Some(libc::EPERM) => ErrorCode::PermissionDenied,
            _ => ErrorCode::OpenFailed,
        };
        let hint = if code == ErrorCode::PermissionDenied {
            ". Install packaging/70-gsp370.rules (or join the uucp group)."
        } else {
            ""
        };
        return Err(Error::new(
            code,
            format!("Cannot open {path}: {err}{hint}"),
        ));
    }

    let file = unsafe { File::from_raw_fd(fd) };

    let mut term: libc::termios = unsafe { std::mem::zeroed() };
    if unsafe { libc::tcgetattr(fd, &mut term) } != 0 {
        return Err(Error::new(
            ErrorCode::ConfigFailed,
            format!("Cannot read terminal settings for {path}: {}", io::Error::last_os_error()),
        ));
    }

    unsafe { libc::cfmakeraw(&mut term) };
    term.c_cflag |= libc::CLOCAL | libc::CREAD;
    term.c_cflag &= !(libc::PARENB | libc::CSTOPB | libc::CRTSCTS);
    term.c_cflag = (term.c_cflag & !libc::CSIZE) | libc::CS8;
    term.c_iflag &= !(libc::IXON | libc::IXOFF | libc::IXANY);
    term.c_cc[libc::VMIN] = 0;
    term.c_cc[libc::VTIME] = 0;

    if unsafe { libc::cfsetispeed(&mut term, libc::B921600) } != 0
        || unsafe { libc::cfsetospeed(&mut term, libc::B921600) } != 0
        || unsafe { libc::tcsetattr(fd, libc::TCSANOW, &term) } != 0
    {
        return Err(Error::new(
            ErrorCode::ConfigFailed,
            format!("Cannot set 921600 8N1 on {path}: {}", io::Error::last_os_error()),
        ));
    }

    unsafe { libc::tcflush(fd, libc::TCIFLUSH) };
    thread::sleep(Duration::from_millis(150));

    Ok(Device { file, path: path.to_owned() })
}

impl Device {
    pub fn open(override_path: Option<&str>) -> Result<Self, Error> {
        let path = override_path
            .map(str::to_owned)
            .or_else(|| std::env::var("GSP370_TTY").ok().filter(|s| !s.is_empty()))
            .or_else(find_port)
            .ok_or_else(|| {
                Error::new(
                    ErrorCode::NoDevice,
                    "GSP 370 dongle not found (USB 1395:009a). Is it plugged in?",
                )
            })?;
        open_path(&path)
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    fn wait(&self, events: libc::c_short, timeout: Duration) -> io::Result<bool> {
        let mut pfd = libc::pollfd {
            fd: self.file.as_raw_fd(),
            events,
            revents: 0,
        };
        let timeout_ms = timeout.as_millis().min(i32::MAX as u128) as i32;
        let ready = unsafe { libc::poll(&mut pfd, 1, timeout_ms) };
        if ready < 0 {
            let err = io::Error::last_os_error();
            if err.kind() == io::ErrorKind::Interrupted {
                return Ok(false);
            }
            return Err(err);
        }
        Ok(ready > 0)
    }

    fn write_all(&self, bytes: &[u8]) -> Result<(), Error> {
        let fd = self.file.as_raw_fd();
        let deadline = Instant::now() + Duration::from_millis(500);
        let mut written = 0;
        while written < bytes.len() {
            let n = unsafe {
                libc::write(
                    fd,
                    bytes[written..].as_ptr().cast(),
                    bytes.len() - written,
                )
            };
            if n > 0 {
                written += n as usize;
                continue;
            }
            let err = io::Error::last_os_error();
            match err.kind() {
                io::ErrorKind::Interrupted => continue,
                io::ErrorKind::WouldBlock => {
                    let remaining = deadline.saturating_duration_since(Instant::now());
                    if remaining.is_zero() {
                        break;
                    }
                    self.wait(libc::POLLOUT, remaining)
                        .map_err(|e| Error::new(ErrorCode::WriteFailed, e.to_string()))?;
                }
                _ => {
                    return Err(Error::new(
                        ErrorCode::WriteFailed,
                        format!("Cannot write to {}: {err}", self.path),
                    ))
                }
            }
        }
        if written != bytes.len() {
            return Err(Error::new(
                ErrorCode::WriteFailed,
                format!("Timed out writing to {}", self.path),
            ));
        }
        Ok(())
    }

    fn command(&self, opcode: u8, params: &[u8]) -> Result<Vec<u8>, Error> {
        thread::sleep(Duration::from_millis(100));
        self.write_all(&make_command(opcode, params))?;

        let fd = self.file.as_raw_fd();
        let deadline = Instant::now() + Duration::from_millis(1500);
        let mut received = Vec::with_capacity(64);
        let mut buf = [0u8; 256];

        loop {
            if let Some(reply) = parse_reply(&received, opcode) {
                return match reply.status {
                    STATUS_OK => Ok(reply.values),
                    STATUS_HEADSET_OFF => Err(Error::new(
                        ErrorCode::HeadsetOff,
                        "The headset is switched off or out of range.",
                    )),
                    status => Err(Error::new(
                        ErrorCode::Protocol,
                        format!(
                            "GSP 370 {} command failed with status 0x{status:02x}.",
                            opcode_name(opcode)
                        ),
                    )),
                };
            }

            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                break;
            }
            let ready = self
                .wait(libc::POLLIN, remaining)
                .map_err(|e| Error::new(ErrorCode::ReadFailed, e.to_string()))?;
            if !ready {
                break;
            }

            let n = unsafe { libc::read(fd, buf.as_mut_ptr().cast(), buf.len()) };
            if n > 0 {
                received.extend_from_slice(&buf[..n as usize]);
                if received.len() > 4096 {
                    let keep = received.len() - 4096;
                    received.drain(..keep);
                }
                continue;
            }
            let err = io::Error::last_os_error();
            match err.kind() {
                io::ErrorKind::Interrupted | io::ErrorKind::WouldBlock => continue,
                _ if n == 0 => {
                    return Err(Error::new(
                        ErrorCode::ReadFailed,
                        format!("The dongle closed the serial port ({}).", self.path),
                    ))
                }
                _ => {
                    return Err(Error::new(
                        ErrorCode::ReadFailed,
                        format!("Cannot read from {}: {err}", self.path),
                    ))
                }
            }
        }

        Err(Error::new(
            ErrorCode::Timeout,
            format!(
                "No reply to the {} command. The headset may be off or out of range.",
                opcode_name(opcode)
            ),
        ))
    }

    pub fn battery(&self) -> Result<u8, Error> {
        let values = self.command(OP_BATTERY, &[])?;
        if values.len() != 1 {
            return Err(Error::new(
                ErrorCode::Protocol,
                format!("Unexpected battery reply ({} value bytes).", values.len()),
            ));
        }
        if values[0] > 100 {
            return Err(Error::new(
                ErrorCode::Protocol,
                format!("Battery level is unavailable (0x{:02x}).", values[0]),
            ));
        }
        Ok(values[0])
    }

    pub fn set_sidetone(&self, percent: u8) -> Result<(), Error> {
        let percent = percent.min(100);
        let level = sidetone_level(percent);
        self.command(if level == 0 { OP_SIDETONE_OFF } else { OP_SIDETONE_ON }, &[])?;
        self.command(OP_SIDETONE_SET, &[level])?;
        Ok(())
    }
}
