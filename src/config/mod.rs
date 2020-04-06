#![allow(dead_code)]

use std::{
    sync::mpsc::{self, Receiver},
    time::Duration,
    io,
    fmt::{self, Display, Formatter},
};

use serde::Deserialize;
use notify::{RecommendedWatcher, Watcher, RecursiveMode, DebouncedEvent};

use crate::{
    maths::{Vec2, Rect},
    rendering::layout::{LayoutBlock, LayoutElement},
};

static mut CONFIG: Option<Config> = None;

#[derive(Debug)]
pub enum Error {
    // Config file not found.
    NotFound(&'static str),
    // Couldn't find config directory.
    NoConfigDirectory(&'static str),
    // Validation error.
    Validate(&'static str),
    // IO error reading file.
    Io(io::Error),
    // Deserialization error.
    Ron(ron::de::Error),
    // Watch error.
    Watch(notify::Error),
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::NotFound(_) => None,
            Error::NoConfigDirectory(_) => None,
            Error::Validate(_) => None,
            Error::Io(err) => err.source(),
            Error::Ron(err) => err.source(),
            Error::Watch(err) => err.source(),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Error::NotFound(path) => write!(f, "Couldn't find a config file: {}", path),
            Error::NoConfigDirectory(dir) =>
                write!(f, "Couldn't locate a config directory a config file: {}", dir), 
            Error::Validate(problem) => write!(f, "Error validating config file: {}", problem), 
            Error::Io(err) => write!(f, "Error reading config file: {}", err), 
            Error::Ron(err) => write!(f, "Problem with config file: {}", err), 
            Error::Watch(err) => write!(f, "Error watching config directory: {}", err), 
        }
    }
}

pub struct ConfigWatcher {
    watcher: RecommendedWatcher,
    pub receiver: Receiver<DebouncedEvent>,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub max_notifications: u32,
    // @TODO: this option should be removed.
    pub width: u32,
    pub height: u32,            // Base height.  NOTE: notification windows will generally be resized, ignoring this value.

    // TODO: timeout should be in seconds.
    pub timeout: i32,           // Default timeout.
    pub poll_interval: u64,

    pub debug: bool,
    pub shortcuts: ShortcutsConfig,

    pub layout: LayoutBlock,
}

impl Config {
    // Initialize the config.  This does a two things:
    // - Attempts to locate and load a config file on the machine, and if it can't, then loads the
    // default config.
    // - Attempts to set up a watcher on the config directory to watch for changes, and returns the
    // watcher or None.
    pub fn init() -> Option<ConfigWatcher> {
        unsafe {
            assert!(CONFIG.is_none());
            let cfg = Config::load();
            match cfg {
                Ok(c) => CONFIG = Some(c),
                Err(e) => {
                    println!("Couldn't load a config, so will use default one:\n\t{}", e);
                    CONFIG = Some(Config::default());
                },
            };

            let watch = Config::watch();
            match watch {
                Ok(w) => return Some(w),
                Err(e) => {
                    println!("Couldn't watch config directory for changes, so we won't:\n\t{}", e);
                    return None;
                },
            }
        }
    }

    // Get immutable reference to global config variable.
    pub fn get() -> &'static Config {
        unsafe {
            assert!(CONFIG.is_some());
            // TODO: can as_ref be removed?
            CONFIG.as_ref().unwrap()
        }
    }

    // Get mutable refernce to global config variable.
    pub fn get_mut() -> &'static mut Config {
        unsafe {
            assert!(CONFIG.is_some());
            // TODO: can as_ref be removed?
            CONFIG.as_mut().unwrap()
        }
    }

    // Attempt to load the config again.
    // If we can, then replace the existing config.
    // If we can't, then do nothing.
    pub fn try_reload() {
        match Config::load() {
            Ok(cfg) => unsafe { CONFIG = Some(cfg) },
            Err(e) => println!("Tried to reload the config but couldn't: {}", e),
        }
    }

    // Load config or return error.
    pub fn load() -> Result<Self, Error> {
        let mut cfg_path = match dirs::config_dir() {
            Some(path) => path,
            None => return Err(Error::NoConfigDirectory("Couldn't find $XDG_CONFIG_HOME or $HOME/.config")),
        };

        cfg_path.push("wiry/config.ron");
        let cfg_string = std::fs::read_to_string(cfg_path);
        let cfg_string = match cfg_string {
            Ok(string) => string,
            Err(e) => return Err(Error::Io(e)),
        };

        let config: Result<Self, _> = ron::de::from_str(cfg_string.as_str());
        match config {
            Ok(cfg) => return cfg.validate(),
            Err(e) => return Err(Error::Ron(e)),
        };
    }

    // Watch config directory for changes, and send message to Configwatcher when something
    // happens.
    pub fn watch() -> Result<ConfigWatcher, Error> {
        let (sender, receiver) = mpsc::channel();
        // Duration is a debouncing period.
        let mut watcher = notify::watcher(sender, Duration::from_millis(10)).expect("Unable to spawn file watcher.");

        let mut cfg_path = match dirs::config_dir() {
            Some(path) => path,
            None => return Err(Error::NoConfigDirectory("Couldn't find $XDG_CONFIG_HOME or $HOME/.config")),
        };

        cfg_path.push("wiry");
        let result = watcher.watch(cfg_path.clone(), RecursiveMode::NonRecursive);
        match result {
            Ok(_) => return Ok(ConfigWatcher { watcher, receiver }),
            Err(e) => return Err(Error::Watch(e)),
        };
    }

    // Verify that the config is constructed correctly.
    fn validate(self) -> Result<Self, Error> {
        match &self.layout.params {
            LayoutElement::NotificationBlock(_) => Ok(self),
            _ => Err(Error::Validate("The first LayoutBlock params must be of type NotificationBlock!")),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        let cfg_string = include_str!("../config.ron");
        ron::de::from_str(cfg_string)
            .expect("Failed to parse default config.  Something is fucked up.\n")
    }
}

#[derive(Debug, Deserialize)]
pub struct ShortcutsConfig {
    pub notification_close: u32,
    pub notification_closeall: u32,
    pub notification_pause: u32,
    pub notification_url: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Padding {
    pub left: f64,
    pub right: f64,
    pub top: f64,
    pub bottom: f64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Offset {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Deserialize, Clone)]
pub enum AnchorPosition {
    ML,
    TL,
    MT,
    TR,
    MR,
    BR,
    MB,
    BL,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

impl Color {
    pub fn new(r: f64, g: f64, b: f64, a: f64) -> Self {
        Color { r, g, b, a }
    }
}

impl Padding {
    pub fn new(left: f64, right: f64, top: f64, bottom: f64) -> Self {
        Padding { left, right, top, bottom }
    }

    pub fn width(&self) -> f64 {
        self.left + self.right
    }
    pub fn height(&self) -> f64 {
        self.top + self.bottom
    }
}

impl AnchorPosition {
    pub fn get_pos(&self, rect: &Rect) -> Vec2 {
        match self {
            AnchorPosition::ML => rect.mid_left(),
            AnchorPosition::TL => rect.top_left(),
            AnchorPosition::MT => rect.mid_top(),
            AnchorPosition::TR => rect.top_right(),
            AnchorPosition::MR => rect.mid_right(),
            AnchorPosition::BR => rect.bottom_right(),
            AnchorPosition::MB => rect.mid_bottom(),
            AnchorPosition::BL => rect.bottom_left(),
        }
    }
}


