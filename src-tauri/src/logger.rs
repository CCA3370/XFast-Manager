use chrono::Local;
use once_cell::sync::Lazy;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::sync::Mutex;

const MAX_LOG_SIZE: u64 = 3 * 1024 * 1024; // 3MB
const TRIM_TARGET_SIZE: u64 = 2 * 1024 * 1024; // Trim to 2MB when exceeds max

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Debug = 0,
    Info = 1,
    Error = 2,
}

impl LogLevel {
    fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Info => "INFO",
            LogLevel::Debug => "DEBUG",
            LogLevel::Error => "ERROR",
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum Locale {
    En,
    Zh,
}

impl Default for Locale {
    fn default() -> Self {
        Locale::En
    }
}

/// Log message keys for translation
pub enum LogMsg {
    AppStarted,
    LaunchedWithArgs,
    InstallationStarted,
    Installing,
    InstallationCompleted,
    InstallationFailed,
    AnalysisStarted,
    AnalysisCompleted,
    ScanFailed,
    CannotInstallFromXPlane,
}

impl LogMsg {
    pub fn translate(&self, locale: Locale) -> &'static str {
        match locale {
            Locale::En => match self {
                LogMsg::AppStarted => "XFastInstall started",
                LogMsg::LaunchedWithArgs => "Launched with arguments",
                LogMsg::InstallationStarted => "Installation started",
                LogMsg::Installing => "Installing",
                LogMsg::InstallationCompleted => "Installation completed successfully",
                LogMsg::InstallationFailed => "Failed to install",
                LogMsg::AnalysisStarted => "Analysis started",
                LogMsg::AnalysisCompleted => "Analysis completed",
                LogMsg::ScanFailed => "Failed to scan",
                LogMsg::CannotInstallFromXPlane => "Cannot install from X-Plane directory. Please drag files from outside X-Plane folder",
            },
            Locale::Zh => match self {
                LogMsg::AppStarted => "XFastInstall 已启动",
                LogMsg::LaunchedWithArgs => "通过参数启动",
                LogMsg::InstallationStarted => "开始安装",
                LogMsg::Installing => "正在安装",
                LogMsg::InstallationCompleted => "安装成功完成",
                LogMsg::InstallationFailed => "安装失败",
                LogMsg::AnalysisStarted => "开始分析",
                LogMsg::AnalysisCompleted => "分析完成",
                LogMsg::ScanFailed => "扫描失败",
                LogMsg::CannotInstallFromXPlane => "无法从 X-Plane 目录内安装。请拖入 X-Plane 目录外的文件或压缩包",
            },
        }
    }
}

struct LoggerInner {
    log_path: PathBuf,
    locale: Locale,
    is_first_log: bool,
    min_level: LogLevel,
}

impl LoggerInner {
    fn new() -> Self {
        let log_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("XFastInstall")
            .join("logs");

        // Create log directory if it doesn't exist
        if let Err(e) = fs::create_dir_all(&log_dir) {
            eprintln!("Failed to create log directory: {}", e);
        }

        Self {
            log_path: log_dir.join("xfastinstall.log"),
            locale: Locale::default(),
            is_first_log: true,
            min_level: LogLevel::Info, // Default to Info level
        }
    }

    fn set_locale(&mut self, locale: Locale) {
        self.locale = locale;
    }

    fn get_locale(&self) -> Locale {
        self.locale
    }

    fn set_min_level(&mut self, level: LogLevel) {
        self.min_level = level;
    }

    fn get_min_level(&self) -> LogLevel {
        self.min_level
    }

    fn write_log(
        &mut self,
        level: LogLevel,
        message: &str,
        context: Option<&str>,
        location: Option<&str>,
    ) {
        // Filter by log level
        if level < self.min_level {
            return;
        }

        // Rotate if needed before writing
        self.rotate_if_needed();

        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
        let level_str = level.as_str();
        let ctx = context.map(|c| format!(" [{}]", c)).unwrap_or_default();

        // Add location info for DEBUG level
        let loc = if matches!(level, LogLevel::Debug) {
            location.map(|l| format!(" [{}]", l)).unwrap_or_default()
        } else {
            String::new()
        };

        let line = format!(
            "[{}] [{}]{}{} {}\n",
            timestamp, level_str, ctx, loc, message
        );

        match OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
        {
            Ok(mut file) => {
                // Add newline before first log entry of this session
                if self.is_first_log {
                    let _ = file.write_all(b"\n");
                    self.is_first_log = false;
                }

                if let Err(e) = file.write_all(line.as_bytes()) {
                    eprintln!("Failed to write log: {}", e);
                }
            }
            Err(e) => {
                eprintln!("Failed to open log file: {}", e);
            }
        }
    }

    fn rotate_if_needed(&self) {
        if let Ok(metadata) = fs::metadata(&self.log_path) {
            if metadata.len() > MAX_LOG_SIZE {
                self.trim_log_file();
            }
        }
    }

    fn trim_log_file(&self) {
        // Read entire file
        let content = match fs::read_to_string(&self.log_path) {
            Ok(c) => c,
            Err(_) => return,
        };

        let lines: Vec<&str> = content.lines().collect();
        if lines.is_empty() {
            return;
        }

        // Calculate how many bytes we need to remove
        let current_size = content.len() as u64;
        if current_size <= TRIM_TARGET_SIZE {
            return;
        }

        let bytes_to_remove = current_size - TRIM_TARGET_SIZE;

        // Find how many lines to skip from the beginning
        let mut bytes_counted: u64 = 0;
        let mut lines_to_skip = 0;

        for line in &lines {
            bytes_counted += line.len() as u64 + 1; // +1 for newline
            lines_to_skip += 1;
            if bytes_counted >= bytes_to_remove {
                break;
            }
        }

        // Keep the remaining lines
        let remaining_lines: Vec<&str> = lines.into_iter().skip(lines_to_skip).collect();
        let new_content = remaining_lines.join("\n") + "\n";

        // Write back
        if let Err(e) = fs::write(&self.log_path, new_content) {
            eprintln!("Failed to trim log file: {}", e);
        }
    }

    fn read_recent_lines(&self, count: usize) -> Vec<String> {
        let file = match File::open(&self.log_path) {
            Ok(f) => f,
            Err(_) => return Vec::new(),
        };

        let reader = BufReader::new(file);
        let all_lines: Vec<String> = reader.lines().filter_map(|l| l.ok()).collect();

        // Return last N lines
        let start = if all_lines.len() > count {
            all_lines.len() - count
        } else {
            0
        };

        all_lines[start..].to_vec()
    }

    fn read_all(&self) -> String {
        fs::read_to_string(&self.log_path).unwrap_or_default()
    }

    fn get_log_path(&self) -> PathBuf {
        self.log_path.clone()
    }

    fn get_log_folder(&self) -> PathBuf {
        self.log_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."))
    }
}

static LOGGER: Lazy<Mutex<LoggerInner>> = Lazy::new(|| Mutex::new(LoggerInner::new()));

// Public API

pub fn set_locale(locale_str: &str) {
    if let Ok(mut logger) = LOGGER.lock() {
        let locale = match locale_str {
            "zh" | "zh-CN" | "zh-TW" | "zh-Hans" | "zh-Hant" => Locale::Zh,
            _ => Locale::En,
        };
        logger.set_locale(locale);
    }
}

pub fn get_locale() -> Locale {
    if let Ok(logger) = LOGGER.lock() {
        logger.get_locale()
    } else {
        Locale::default()
    }
}

/// Translate a log message key to the current locale
pub fn tr(msg: LogMsg) -> String {
    let locale = get_locale();
    msg.translate(locale).to_string()
}

pub fn log_info(message: &str, context: Option<&str>) {
    if let Ok(mut logger) = LOGGER.lock() {
        logger.write_log(LogLevel::Info, message, context, None);
    }
}

pub fn log_debug(message: &str, context: Option<&str>, location: Option<&str>) {
    if let Ok(mut logger) = LOGGER.lock() {
        logger.write_log(LogLevel::Debug, message, context, location);
    }
}

pub fn log_error(message: &str, context: Option<&str>) {
    if let Ok(mut logger) = LOGGER.lock() {
        logger.write_log(LogLevel::Error, message, context, None);
    }
}

pub fn set_log_level(level: LogLevel) {
    if let Ok(mut logger) = LOGGER.lock() {
        logger.set_min_level(level);
    }
}

pub fn is_debug_enabled() -> bool {
    if let Ok(logger) = LOGGER.lock() {
        logger.get_min_level() <= LogLevel::Debug
    } else {
        false
    }
}

/// Macro for debug logging with automatic file and line number
#[macro_export]
macro_rules! log_debug {
    ($msg:expr) => {
        $crate::logger::log_debug($msg, None, Some(concat!(file!(), ":", line!())))
    };
    ($msg:expr, $ctx:expr) => {
        $crate::logger::log_debug($msg, Some($ctx), Some(concat!(file!(), ":", line!())))
    };
}

pub fn get_recent_logs(count: usize) -> Vec<String> {
    if let Ok(logger) = LOGGER.lock() {
        logger.read_recent_lines(count)
    } else {
        Vec::new()
    }
}

pub fn get_all_logs() -> String {
    if let Ok(logger) = LOGGER.lock() {
        logger.read_all()
    } else {
        String::new()
    }
}

pub fn get_log_path() -> PathBuf {
    if let Ok(logger) = LOGGER.lock() {
        logger.get_log_path()
    } else {
        PathBuf::new()
    }
}

pub fn get_log_folder() -> PathBuf {
    if let Ok(logger) = LOGGER.lock() {
        logger.get_log_folder()
    } else {
        PathBuf::new()
    }
}
