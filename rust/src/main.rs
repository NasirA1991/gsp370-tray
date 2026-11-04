mod icon;
mod protocol;
mod serial;

use icon::TrayState;
use ksni::blocking::{Handle, TrayMethods};
use ksni::menu::{
    CheckmarkItem, MenuItem, RadioGroup, RadioItem, StandardItem, SubMenu,
};
use ksni::{Category, Status, ToolTip};
use serial::{Device, ErrorCode};

use std::env;
use std::fs::{self, File, OpenOptions};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const APP_ID: &str = "gsp370-tray";

#[derive(Clone, Debug)]
struct Config {
    sidetone: u8,
    alerts: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            sidetone: 50,
            alerts: true,
        }
    }
}

fn config_home() -> PathBuf {
    if let Ok(xdg) = env::var("XDG_CONFIG_HOME") {
        if !xdg.is_empty() {
            return PathBuf::from(xdg);
        }
    }
    PathBuf::from(env::var("HOME").unwrap_or_else(|_| ".".into())).join(".config")
}

fn config_path() -> PathBuf {
    config_home().join(APP_ID).join("config")
}

fn runtime_dir() -> PathBuf {
    env::var("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}

fn load_config() -> Config {
    let mut config = Config::default();
    let Ok(text) = fs::read_to_string(config_path()) else {
        return config;
    };
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key.trim() {
            "sidetone" => {
                if let Ok(v) = value.trim().parse::<u8>() {
                    config.sidetone = v.min(100);
                }
            }
            "alerts" => {
                config.alerts = !matches!(value.trim(), "0" | "false" | "no");
            }
            _ => {}
        }
    }
    config
}

fn save_config(config: &Config) {
    let path = config_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let body = format!(
        "sidetone={}\nalerts={}\n",
        config.sidetone,
        if config.alerts { 1 } else { 0 }
    );
    if let Err(error) = fs::write(&path, body) {
        eprintln!("gsp370-tray: cannot save {}: {error}", path.display());
    }
}

fn autostart_path() -> PathBuf {
    config_home().join("autostart").join("gsp370-tray.desktop")
}

fn autostart_enabled() -> bool {
    autostart_path().is_file()
}

fn desktop_quote(path: &Path) -> String {
    let text = path.to_string_lossy().replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{text}\"")
}

fn write_autostart() -> std::io::Result<()> {
    let path = autostart_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let exec = env::current_exe()?;
    let body = format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Version=1.0\n\
         Name=GSP 370 Tray\n\
         Comment=Battery and sidetone for the Sennheiser/EPOS GSP 370\n\
         Exec={}\n\
         Icon=gsp370-tray\n\
         Terminal=false\n\
         Categories=Utility;\n\
         X-GNOME-Autostart-enabled=true\n",
        desktop_quote(&exec)
    );
    fs::write(path, body)
}

fn remove_autostart() -> std::io::Result<()> {
    match fs::remove_file(autostart_path()) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn notify(title: &str, body: &str) {
    let _ = ProcessCommand::new("notify-send")
        .args(["-a", "GSP 370 Tray", "-i", "gsp370-tray", "-u", "normal", title, body])
        .status();
}

#[derive(Debug)]
enum Command {
    Refresh,
    SetSidetone(u8),
    SetAlerts(bool),
}

struct AppTray {
    battery: Option<u8>,
    state: TrayState,
    detail: String,
    device: String,
    sidetone: u8,
    alerts: bool,
    autostart: bool,
    text_color: [u8; 3],
    tx: Sender<Command>,
}

impl AppTray {
    fn new(tx: Sender<Command>, config: &Config) -> Self {
        Self {
            battery: None,
            state: TrayState::Unknown,
            detail: "Starting…".into(),
            device: String::new(),
            sidetone: config.sidetone,
            alerts: config.alerts,
            autostart: autostart_enabled(),
            text_color: icon::theme_text_color(),
            tx,
        }
    }

    fn status_label(&self) -> String {
        match self.state {
            TrayState::Connected => match self.battery {
                Some(b) => format!("Battery: {b}%"),
                None => "Connected".into(),
            },
            TrayState::Off => "Headset off".into(),
            TrayState::NoDongle => "Dongle not found".into(),
            TrayState::Permission => "Serial permission denied".into(),
            TrayState::Error => "Connection problem".into(),
            TrayState::Unknown => "Starting…".into(),
        }
    }

    fn tooltip(&self) -> String {
        let mut lines = vec![self.status_label()];
        if self.state == TrayState::Connected {
            lines.push(format!("Sidetone: {}% (last set)", self.sidetone));
        }
        if !self.detail.is_empty() {
            lines.push(self.detail.clone());
        }
        lines.join("\n")
    }

    fn sidetone_index(&self) -> usize {
        if self.sidetone == 0 {
            0
        } else {
            ((self.sidetone / 10).clamp(1, 10)) as usize
        }
    }
}

impl ksni::Tray for AppTray {
    const MENU_ON_ACTIVATE: bool = true;

    fn id(&self) -> String {
        APP_ID.into()
    }

    fn title(&self) -> String {
        "GSP 370 Tray".into()
    }

    fn category(&self) -> Category {
        // KDE groups tray entries by category.  ApplicationStatus places this
        // next to ordinary apps instead of under "Hardware", which also moves
        // it left in the tray's default ordering.
        Category::ApplicationStatus
    }

    fn status(&self) -> Status {
        // Plasma's "only show when relevant" mode hides Passive items.
        // The headset being off is exactly the not-relevant state.
        if self.state == TrayState::Off {
            return Status::Passive;
        }
        if self.state == TrayState::Connected
            && self.battery.is_some_and(|b| b <= 10)
        {
            Status::NeedsAttention
        } else {
            Status::Active
        }
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        icon::icons(self.state, self.battery, self.text_color)
    }

    fn tool_tip(&self) -> ToolTip {
        ToolTip {
            title: "GSP 370".into(),
            description: self.tooltip(),
            ..Default::default()
        }
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let connected = self.state == TrayState::Connected;
        let mut items: Vec<MenuItem<Self>> = Vec::with_capacity(10);

        items.push(
            StandardItem {
                label: self.status_label(),
                enabled: false,
                ..Default::default()
            }
            .into(),
        );
        if !self.detail.is_empty() {
            items.push(
                StandardItem {
                    label: self.detail.clone(),
                    enabled: false,
                    ..Default::default()
                }
                .into(),
            );
        }

        items.push(MenuItem::Separator);

        let levels = [0u8, 10, 20, 30, 40, 50, 60, 70, 80, 90, 100];
        let options = levels
            .iter()
            .map(|&percent| RadioItem {
                label: if percent == 0 {
                    "Off".into()
                } else {
                    format!("{percent}%")
                },
                enabled: connected,
                ..Default::default()
            })
            .collect();

        items.push(
            SubMenu {
                label: "Sidetone".into(),
                enabled: connected,
                submenu: vec![
                    RadioGroup {
                        selected: self.sidetone_index(),
                        select: Box::new(|this: &mut Self, index: usize| {
                            let percent = if index == 0 { 0 } else { index as u8 * 10 };
                            let _ = this.tx.send(Command::SetSidetone(percent));
                        }),
                        options,
                    }
                    .into(),
                ],
                ..Default::default()
            }
            .into(),
        );

        items.push(MenuItem::Separator);
        items.push(
            StandardItem {
                label: "Refresh now".into(),
                activate: Box::new(|this: &mut Self| {
                    let _ = this.tx.send(Command::Refresh);
                }),
                ..Default::default()
            }
            .into(),
        );
        items.push(
            CheckmarkItem {
                label: "Low battery alerts".into(),
                checked: self.alerts,
                activate: Box::new(|this: &mut Self| {
                    this.alerts = !this.alerts;
                    let _ = this.tx.send(Command::SetAlerts(this.alerts));
                }),
                ..Default::default()
            }
            .into(),
        );
        items.push(
            CheckmarkItem {
                label: "Start at login".into(),
                checked: self.autostart,
                activate: Box::new(|this: &mut Self| {
                    let target = !this.autostart;
                    let result = if target {
                        write_autostart()
                    } else {
                        remove_autostart()
                    };
                    match result {
                        Ok(()) => this.autostart = target,
                        Err(error) => {
                            this.detail = format!("Autostart failed: {error}");
                        }
                    }
                }),
                ..Default::default()
            }
            .into(),
        );
        items.push(MenuItem::Separator);
        items.push(
            StandardItem {
                label: "Quit".into(),
                activate: Box::new(|_| std::process::exit(0)),
                ..Default::default()
            }
            .into(),
        );
        items
    }

    fn menu_about_to_show(&mut self) {
        let _ = self.tx.send(Command::Refresh);
    }

    fn secondary_activate(&mut self, _x: i32, _y: i32) {
        let _ = self.tx.send(Command::Refresh);
    }
}

enum Probe {
    Connected { battery: u8, path: String },
    Off,
    NoDevice(String),
    Permission(String),
    Error(String),
}

fn probe(device_override: Option<&str>) -> Probe {
    let device = match Device::open(device_override) {
        Ok(device) => device,
        Err(error) => {
            return match error.code {
                ErrorCode::NoDevice => Probe::NoDevice(error.message),
                ErrorCode::PermissionDenied => Probe::Permission(error.message),
                _ => Probe::Error(error.message),
            };
        }
    };

    match device.battery() {
        Ok(battery) => Probe::Connected {
            battery,
            path: device.path().to_owned(),
        },
        Err(error) => match error.code {
            ErrorCode::HeadsetOff => Probe::Off,
            ErrorCode::NoDevice => Probe::NoDevice(error.message),
            ErrorCode::PermissionDenied => Probe::Permission(error.message),
            _ => Probe::Error(error.message),
        },
    }
}

fn poll_interval(state: TrayState) -> Duration {
    match state {
        TrayState::Connected => Duration::from_secs(60),
        TrayState::Off => Duration::from_secs(20),
        TrayState::NoDongle => Duration::from_secs(15),
        TrayState::Permission => Duration::from_secs(60),
        TrayState::Error => Duration::from_secs(30),
        TrayState::Unknown => Duration::from_secs(5),
    }
}

fn update_from_probe(
    handle: &Handle<AppTray>,
    outcome: Probe,
    config: &Config,
    last_battery: &mut Option<u8>,
    low_notified: &mut u8,
) -> TrayState {
    let (state, battery, detail, device_path) = match outcome {
        Probe::Connected { battery, path } => {
            if battery > 25 {
                *low_notified = 0;
            }
            if let Some(previous) = *last_battery {
                if battery > previous.saturating_add(2) {
                    *low_notified = 0;
                }
            }
            *last_battery = Some(battery);

            if config.alerts {
                for threshold in [20u8, 10, 5] {
                    let bit = match threshold {
                        20 => 1,
                        10 => 2,
                        _ => 4,
                    };
                    if battery <= threshold && (*low_notified & bit) == 0 {
                        *low_notified |= bit;
                        notify(
                            "GSP 370 battery low",
                            &format!("Battery is at {battery}% — charge the headset."),
                        );
                        break;
                    }
                }
            }

            (
                TrayState::Connected,
                Some(battery),
                format!("via {path}"),
                path,
            )
        }
        Probe::Off => (
            TrayState::Off,
            None,
            "Turn the headset on to read its battery.".into(),
            String::new(),
        ),
        Probe::NoDevice(error) => (
            TrayState::NoDongle,
            None,
            error,
            String::new(),
        ),
        Probe::Permission(error) => (
            TrayState::Permission,
            None,
            error,
            String::new(),
        ),
        Probe::Error(error) => (
            TrayState::Error,
            None,
            error,
            String::new(),
        ),
    };

    let state_for_return = state;
    let updated = handle.update(|tray| {
        tray.state = state;
        tray.battery = battery;
        tray.detail = detail;
        tray.device = device_path;
        tray.alerts = config.alerts;
        tray.autostart = autostart_enabled();
        tray.text_color = icon::theme_text_color();
    });
    if updated.is_none() {
        std::process::exit(0);
    }
    state_for_return
}

fn serial_loop(
    rx: Receiver<Command>,
    handle: Handle<AppTray>,
    mut config: Config,
    device_override: Option<String>,
) {
    let mut next_poll = Instant::now() + Duration::from_millis(250);
    let mut last_battery: Option<u8> = None;
    let mut low_notified: u8 = 0;

    loop {
        let now = Instant::now();
        let wait = next_poll.saturating_duration_since(now);
        let command = if wait.is_zero() {
            None
        } else {
            match rx.recv_timeout(wait) {
                Ok(command) => Some(command),
                Err(mpsc::RecvTimeoutError::Timeout) => None,
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        };

        if let Some(command) = command {
            match command {
                Command::Refresh => {
                    next_poll = Instant::now();
                }
                Command::SetAlerts(enabled) => {
                    config.alerts = enabled;
                    save_config(&config);
                    let _ = handle.update(|tray| tray.alerts = enabled);
                }
                Command::SetSidetone(percent) => {
                    let result = Device::open(device_override.as_deref())
                        .and_then(|device| device.set_sidetone(percent));
                    match result {
                        Ok(()) => {
                            config.sidetone = percent;
                            save_config(&config);
                            let _ = handle.update(|tray| {
                                tray.sidetone = percent;
                                tray.detail = format!("Sidetone set to {percent}%");
                            });
                            next_poll = Instant::now() + Duration::from_millis(300);
                        }
                        Err(error) => {
                            let detail = error.message.clone();
                            let state = if error.code == ErrorCode::HeadsetOff {
                                Some(TrayState::Off)
                            } else {
                                None
                            };
                            let _ = handle.update(|tray| {
                                if let Some(state) = state {
                                    tray.state = state;
                                    tray.battery = None;
                                }
                                tray.detail = detail;
                            });
                            next_poll = Instant::now() + Duration::from_millis(500);
                        }
                    }
                }
            }
        }

        if Instant::now() >= next_poll {
            let state = update_from_probe(
                &handle,
                probe(device_override.as_deref()),
                &config,
                &mut last_battery,
                &mut low_notified,
            );
            next_poll = Instant::now() + poll_interval(state);
        }
    }
}

fn acquire_single_instance() -> Option<File> {
    let path = runtime_dir().join("gsp370-tray.lock");
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .ok()?;
    let rc = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    if rc == 0 { Some(file) } else { None }
}

fn cli_device(args: &[String]) -> Option<String> {
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == "--device" || arg == "-d" {
            return iter.next().cloned();
        }
        if let Some(value) = arg.strip_prefix("--device=") {
            return Some(value.to_owned());
        }
    }
    None
}

fn cli_set_sidetone(args: &[String]) -> Option<Result<u8, String>> {
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == "--set-sidetone" || arg == "-s" {
            let Some(value) = iter.next() else {
                return Some(Err("missing --set-sidetone value".into()));
            };
            return Some(parse_percent(value));
        }
        if let Some(value) = arg.strip_prefix("--set-sidetone=") {
            return Some(parse_percent(value));
        }
    }
    None
}

fn parse_percent(value: &str) -> Result<u8, String> {
    let percent = value
        .parse::<u8>()
        .map_err(|_| "sidetone must be an integer from 0 to 100".to_owned())?;
    if percent > 100 {
        return Err("sidetone must be an integer from 0 to 100".into());
    }
    Ok(percent)
}

fn print_help() {
    println!(
        "gsp370-tray {VERSION}\n\
         Battery and sidetone tray for the Sennheiser/EPOS GSP 370.\n\n\
         Usage: gsp370-tray [options]\n\n\
         Options:\n\
           -b, --battery              print battery percentage and exit\n\
           -p, --probe                print device, link and battery status\n\
           -s, --set-sidetone <0-100> set sidetone and exit (0 = off)\n\
           -d, --device <path>        override serial device (for tests)\n\
           -h, --help                 show this help\n\
           -v, --version              show version\n\n\
         Without options, the StatusNotifierItem tray starts."
    );
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let cli = args.iter().any(|arg| {
        matches!(
            arg.as_str(),
            "--battery"
                | "-b"
                | "--probe"
                | "-p"
                | "--set-sidetone"
                | "-s"
                | "--help"
                | "-h"
                | "--version"
                | "-v"
        ) || arg.starts_with("--set-sidetone=")
    });

    if cli {
        if args.iter().any(|arg| arg == "--help" || arg == "-h") {
            print_help();
            return;
        }
        if args.iter().any(|arg| arg == "--version" || arg == "-v") {
            println!("gsp370-tray {VERSION}");
            return;
        }
        let device = cli_device(&args);
        if let Some(result) = cli_set_sidetone(&args) {
            let percent = match result {
                Ok(percent) => percent,
                Err(error) => {
                    eprintln!("gsp370-tray: {error}");
                    std::process::exit(1);
                }
            };
            match Device::open(device.as_deref()).and_then(|d| d.set_sidetone(percent)) {
                Ok(()) => println!("sidetone={percent}%"),
                Err(error) => {
                    eprintln!("gsp370-tray: {}", error.message);
                    std::process::exit(if error.code == ErrorCode::HeadsetOff { 2 } else { 1 });
                }
            }
            return;
        }
        if args.iter().any(|arg| arg == "--battery" || arg == "-b") {
            match Device::open(device.as_deref()).and_then(|d| d.battery()) {
                Ok(percent) => println!("{percent}"),
                Err(error) => {
                    if error.code == ErrorCode::HeadsetOff {
                        println!("off");
                        eprintln!("gsp370-tray: {}", error.message);
                        std::process::exit(2);
                    }
                    eprintln!("gsp370-tray: {}", error.message);
                    std::process::exit(1);
                }
            }
            return;
        }
        if args.iter().any(|arg| arg == "--probe" || arg == "-p") {
            match Device::open(device.as_deref()) {
                Ok(device) => {
                    println!("device={}", device.path());
                    match device.battery() {
                        Ok(percent) => {
                            println!("status=on");
                            println!("battery={percent}");
                        }
                        Err(error) if error.code == ErrorCode::HeadsetOff => {
                            println!("status=headset-off");
                            std::process::exit(2);
                        }
                        Err(error) => {
                            println!("status={:?}", error.code);
                            eprintln!("gsp370-tray: {}", error.message);
                            std::process::exit(1);
                        }
                    }
                }
                Err(error) => {
                    println!("status={:?}", error.code);
                    eprintln!("gsp370-tray: {}", error.message);
                    std::process::exit(1);
                }
            }
            return;
        }
        print_help();
        return;
    }

    let Some(_lock) = acquire_single_instance() else {
        eprintln!("gsp370-tray: another instance is already running");
        std::process::exit(0);
    };

    let config = load_config();
    let (tx, rx) = mpsc::channel();
    let tray = AppTray::new(tx, &config);
    let handle = match tray.spawn() {
        Ok(handle) => handle,
        Err(error) => {
            eprintln!("gsp370-tray: cannot start StatusNotifierItem: {error}");
            std::process::exit(1);
        }
    };

    let serial_handle = handle.clone();
    let device = cli_device(&args);
    let serial_config = config.clone();
    thread::spawn(move || serial_loop(rx, serial_handle, serial_config, device));

    loop {
        thread::park();
    }
}
