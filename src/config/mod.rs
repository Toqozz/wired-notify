#![allow(dead_code)]

use serde::Deserialize;

use crate::types::maths::{Vec2, Rect};
use crate::rendering::layout::{
    LayoutBlock, Hook,
    LayoutElement::{
        NotificationBlock,
        TextBlock,
        ScrollingTextBlock,
        ImageBlock,
    },
};

use notify::{RecommendedWatcher, Watcher, RecursiveMode, DebouncedEvent, watcher};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

// @TODO: do some stuff to verify the config at runtime.
// i.e. check that the first block is a notificationblock.
#[derive(Debug, Deserialize)]
pub struct Config {
    pub max_notifications: u32,
    pub width: u32,
    pub height: u32,            // Base height.  NOTE: notification windows will generally be resized, ignoring this value.

    // TODO: timeout should be in seconds.
    pub timeout: i32,           // Default timeout.
    pub poll_interval: u64,

    pub debug: bool,
    pub shortcuts: ShortcutsConfig,

    pub layout: LayoutBlock,

    // Runtime useful things related to configuration.
    //#[serde(skip)]
    //pub monitor: Option<winit::monitor::MonitorHandle>,
}

impl Config {
    pub fn load() -> Self {
        let cfg: Self;

        if let Some(mut cfg_path) = dirs::config_dir() {
            cfg_path.push("wiry/config.ron");

            if let Ok(cfg_string) = std::fs::read_to_string(cfg_path.clone()) {
                println!("Loading config: {}.", &cfg_path.to_string_lossy());
                cfg = ron::de::from_str(cfg_string.as_str())
                    .expect("Found a config, but failed to read it.\n");
            } else {
                println!("Couldn't find the config file: {}; using default config.", &cfg_path.to_string_lossy());
                cfg = Config::default();
            }
        } else {
            println!("Couldn't find the config directory: {}; using default config.", "$XDG_CONFIG_HOME or $HOME/.config");
            cfg = Config::default();
        }

        cfg
    }

    pub fn watch() -> Option<(RecommendedWatcher, Receiver<DebouncedEvent>)> {
        let (sx, rx) = mpsc::channel();
        // Duration is a debouncing period.
        let mut watcher = notify::watcher(sx, Duration::from_millis(10)).expect("Unable to spawn file watcher.");

        // @TODO: this needs to handle when the directory doesn't exist.
        if let Some(mut cfg_path) = dirs::config_dir() {
            cfg_path.push("wiry");
            let result = watcher.watch(cfg_path.clone(), RecursiveMode::NonRecursive);
            match result {
                Ok(_) => return Some((watcher, rx)),
                Err(_) => {
                    println!("There is no directory: {}, so won't watch for config changes.", &cfg_path.to_string_lossy());
                    return None;
                }
            }
        } else {
            println!("Couldn't find a config directory: {}; so won't watch for changes.", "$XDG_CONFIG_HOME or $HOME/.config");
            None
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


