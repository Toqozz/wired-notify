#![allow(dead_code)]

use std::{
    env,
    fmt::{self, Display, Formatter},
    io,
    path::PathBuf,
    sync::mpsc::{self, Receiver},
    time::Duration,
};

use notify::{DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{
    de::{self, Deserializer, Unexpected},
    Deserialize,
};

use crate::{
    maths_utility::{self, Rect, Vec2},
    rendering::layout::{LayoutBlock, LayoutElement},
};

static mut CONFIG: Option<Config> = None;

#[derive(Debug)]
pub enum Error {
    // Config file not found.
    NotFound,
    // Validation error.
    Validate(&'static str),
    // Bad hex string error.
    Hexidecimal(&'static str),
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
            Error::NotFound => None,
            Error::Validate(_) => None,
            Error::Hexidecimal(_) => None,
            Error::Io(err) => err.source(),
            Error::Ron(err) => err.source(),
            Error::Watch(err) => err.source(),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Error::NotFound => write!(f, "No config found"),
            Error::Validate(problem) => write!(f, "Error validating config file: {}", problem),
            Error::Hexidecimal(problem) => {
                write!(f, "Error parsing hexidecimal string: {}", problem)
            }
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
    // Maximum number of notifications to show on screen at once.
    pub max_notifications: usize,
    pub timeout: i32,                  // Default timeout, in milliseconds.
    pub poll_interval: u64,            // "Frame rate" / check for updates and new notifications.
    pub print_to_file: Option<String>, // A file to print notification info to, for scripting purposes.
    pub idle_threshold: Option<u64>,   // The threshold before pausing notifications due to being idle.  0 = ignore.
    pub layout_blocks: Vec<LayoutBlock>,

    // Optional Properties
    // Draws rectangles around elements.
    #[serde(default)]
    pub debug: bool,
    #[serde(default = "Config::default_debug_color")]
    pub debug_color: Color,
    #[serde(default = "Config::default_debug_color_alt")]
    pub debug_color_alt: Color,

    // Enable/disable notification replace functionality.  I don't like how some apps do it.
    #[serde(default = "maths_utility::val_true")]
    pub replacing_enabled: bool,
    // Whether or not to refresh the timeout of a notification on an update
    #[serde(default)]
    pub replacing_resets_timeout: bool,
    // Enable/disable notification closing functionality.  I don't like how some apps do it.
    #[serde(default = "maths_utility::val_true")]
    pub closing_enabled: bool,
    // How many notifications are kept in history.  Older notifications will be removed first.
    // Each notification is roughly 256 bytes (excluding buffers), so do the math there.
    #[serde(default = "maths_utility::val_10")]
    pub history_length: usize,

    // Minimum window width and height.  This is used to create the base rect that the notification
    // grows within.
    // The notification window will never be smaller than this.
    // A value of 1 means that the window will generally always resize with notification, unless
    // you have a 1x1 pixel notification...
    #[serde(default)]
    pub min_window_width: u32,
    #[serde(default)]
    pub min_window_height: u32,

    #[serde(default)]
    pub shortcuts: ShortcutsConfig,

    #[serde(skip)]
    pub layout: Option<LayoutBlock>,
}

impl Config {
    pub fn default_debug_color() -> Color {
        Color::from_rgba(0.0, 1.0, 0.0, 1.0)
    }

    pub fn default_debug_color_alt() -> Color {
        Color::from_rgba(1.0, 0.0, 0.0, 1.0)
    }

    // Initialize the config.  This does a two things:
    // - Attempts to locate and load a config file on the machine, and if it can't, then loads the
    // default config.
    // - If config was loaded successfully, then sets up a watcher on the config file to watch for changes,
    // and returns the watcher or None.
    pub fn init() -> Option<ConfigWatcher> {
        fn assign_config(cfg: Config) {
            unsafe {
                CONFIG = Some(cfg);
            }
        }

        unsafe {
            assert!(CONFIG.is_none());
        }
        let cfg_file = Config::installed_config();
        match cfg_file {
            Some(f) => {
                let cfg = Config::load_file(f.clone());
                match cfg {
                    Ok(c) => assign_config(c),
                    Err(e) => {
                        println!(
                            "Found a config file: {}, but couldn't load it, so will \
                                    use default one for now.\n\
                                    If you fix the error the config will be reloaded automatically.\n\
                                    \tError: {}\n",
                            f.to_str().unwrap(),
                            e
                        );

                        assign_config(Config::default());
                    }
                }

                // Watch the config file directory for changes, even if it didn't load correctly; we
                // assume that the config we found is the one we're using.
                // It would be nice to be able to watch the config directories for when a user
                // creates a config, but it doesn't seem worthwhile to watch that many directories.
                //
                // NOTE: watching the directory can actually cause us to try and read all file
                // changes in this directory, so we need to remember to check the filename
                // before reloading.
                let watch = Config::watch(f);
                match watch {
                    Ok(w) => Some(w),
                    Err(e) => {
                        println!(
                            "There was a problem watching the config for changes; so won't watch:\n\t{}",
                            e
                        );
                        None
                    }
                }
            }

            None => {
                println!("Couldn't load a config because we couldn't find one, so will use default.");
                assign_config(Config::default());
                None
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
    pub fn try_reload(path: PathBuf) -> bool {
        match Config::load_file(path) {
            Ok(cfg) => {
                unsafe {
                    CONFIG = Some(cfg);
                }
                println!("Config reloaded.");
                true
            }
            Err(e) => {
                println!("Tried to reload the config but couldn't: {}", e);
                false
            }
        }
    }

    // https://github.com/alacritty/alacritty/blob/f14d24542c3ceda3b508c707eb79cf2fe2a04bd1/alacritty/src/config/mod.rs#L98
    fn installed_config() -> Option<PathBuf> {
        xdg::BaseDirectories::with_prefix("wired")
            .ok()
            .and_then(|xdg| xdg.find_config_file("wired.ron"))
            .or_else(|| {
                xdg::BaseDirectories::new()
                    .ok()
                    .and_then(|fallback| fallback.find_config_file("wired.ron"))
            })
            .or_else(|| {
                if let Ok(home) = env::var("HOME") {
                    // Fallback path: `$HOME/.config/wired/wired.ron`
                    let fallback = PathBuf::from(&home).join(".config/wired/wired.ron");
                    if fallback.exists() {
                        return Some(fallback);
                    }

                    // Fallback path: `$HOME/.wired.ron`
                    let fallback = PathBuf::from(&home).join(".wired.ron");
                    if fallback.exists() {
                        return Some(fallback);
                    }
                }

                None
            })
    }

    // Load config or return error.
    pub fn load_file(path: PathBuf) -> Result<Self, Error> {
        let cfg_string = std::fs::read_to_string(path);
        let cfg_string = match cfg_string {
            Ok(mut string) => {
                string.insert_str(0, "#![enable(implicit_some)]\n");
                string
            }
            Err(e) => return Err(Error::Io(e)),
        };

        Config::load_str(cfg_string.as_str())
    }

    pub fn load_str(cfg_str: &str) -> Result<Self, Error> {
        // Really ugly and annoying hack because ron doesn't allow implicit some by
        // default.
        let string = format!("#![enable(implicit_some)]\n{}", cfg_str);
        let config: Result<Self, _> = ron::de::from_str(string.as_str());
        match config {
            Ok(cfg) => Config::transform_and_validate(cfg),
            Err(e) => Err(Error::Ron(e)),
        }
    }

    pub fn transform_and_validate(mut config: Config) -> Result<Self, Error> {
        // NOTE: we might actually want to search for the "root" text.
        if config.layout_blocks.is_empty() {
            return Err(Error::Validate("Config did not contain any layout blocks!"));
        }

        // Look for children of current root.
        // If child found, insert it and then look for children of that node.
        let mut blocks = config.layout_blocks;
        let mut root = blocks.swap_remove(0);
        config.layout_blocks = vec![]; // "Take" vec from config.

        fn find_and_add_children(
            cur_root: &mut LayoutBlock,
            mut remaining: Vec<LayoutBlock>,
        ) -> Vec<LayoutBlock> {
            let mut i = 0;
            while i < remaining.len() {
                if remaining[i].parent == cur_root.name {
                    let mut block = remaining.swap_remove(i);
                    remaining = find_and_add_children(&mut block, remaining);
                    cur_root.children.push(block);

                    // Back to beginning, as remaining has certainly changed and our information is
                    // outdated.
                    // There's surely a better way of doing this, but it works fine for now.
                    i = 0;
                } else {
                    i += 1;
                }
            }

            remaining
        }

        let remaining = find_and_add_children(&mut root, blocks);
        if !remaining.is_empty() && config.debug {
            eprintln!("There were {} blocks remaining after creating the layout tree.  Something must be wrong here.", remaining.len());
        }

        match root.params {
            LayoutElement::NotificationBlock(_) => {
                config.layout = Some(root);
                Ok(config)
            }
            _ => Err(Error::Validate(
                "The first LayoutBlock params must be of type NotificationBlock!",
            )),
        }
    }

    // Watch config file for changes, and send message to `Configwatcher` when something
    // happens.
    pub fn watch(mut path: PathBuf) -> Result<ConfigWatcher, Error> {
        let (sender, receiver) = mpsc::channel();

        // Duration is a debouncing period.
        let mut watcher =
            notify::watcher(sender, Duration::from_millis(10)).expect("Unable to spawn file watcher.");

        // Watch dir.
        path.pop();
        let path = std::fs::canonicalize(path).expect("Couldn't canonicalize path, wtf.");
        let result = watcher.watch(path, RecursiveMode::NonRecursive);
        match result {
            Ok(_) => Ok(ConfigWatcher { watcher, receiver }),
            Err(e) => Err(Error::Watch(e)),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Config::load_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/wired.ron")))
            .expect("Failed to load default config.  Maintainer fucked something up.\n")
    }
}

#[derive(Debug, Deserialize)]
pub struct ShortcutsConfig {
    pub notification_interact: Option<u16>,
    pub notification_close: Option<u16>,
    pub notification_closeall: Option<u16>,
    pub notification_pause: Option<u16>,

    pub notification_action1: Option<u16>,
    pub notification_action2: Option<u16>,
    pub notification_action3: Option<u16>,
    pub notification_action4: Option<u16>,

    pub notification_action1_and_close: Option<u16>,
    pub notification_action2_and_close: Option<u16>,
    pub notification_action3_and_close: Option<u16>,
    pub notification_action4_and_close: Option<u16>,
}

impl Default for ShortcutsConfig {
    fn default() -> Self {
        Self {
            notification_interact: Some(1),
            notification_close: Some(2),
            notification_closeall: Some(7),
            notification_pause: None,

            notification_action1: Some(3),
            notification_action2: None,
            notification_action3: None,
            notification_action4: None,

            notification_action1_and_close: None,
            notification_action2_and_close: None,
            notification_action3_and_close: None,
            notification_action4_and_close: None,
        }
    }
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

#[derive(Debug, Clone)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

impl Color {
    pub fn from_rgba(r: f64, g: f64, b: f64, a: f64) -> Self {
        Color { r, g, b, a }
    }

    pub fn from_hex(hex: &str) -> Result<Self, Error> {
        // Sanitize string a little.
        // Works for strings in format: "#ff000000", "#0xff000000", "0xff000000".
        // We also support hex strings that don't specify alpha: "#000000"
        let sanitized = hex.trim_start_matches('#').trim_start_matches("0x");

        // Convert string to base-16 u32.
        let dec = u32::from_str_radix(sanitized, 16);
        let dec = match dec {
            Ok(d) => d,
            Err(_) => return Err(Error::Hexidecimal("Invalid hexidecimal string.")),
        };

        // If we have 8 chars, then this is hex string includes alpha, if we have 6, then it
        // doesn't.  Anything else at this point is invalid.
        let len = sanitized.chars().count();
        if len == 8 {
            let a = ((dec >> 24) & 0xff) as f64 / 255.0;
            let r = ((dec >> 16) & 0xff) as f64 / 255.0;
            let g = ((dec >> 8) & 0xff) as f64 / 255.0;
            let b = (dec & 0xff) as f64 / 255.0;
            Ok(Color::from_rgba(r, g, b, a))
        } else if len == 6 {
            let a = 1.0;
            let r = ((dec >> 16) & 0xff) as f64 / 255.0;
            let g = ((dec >> 8) & 0xff) as f64 / 255.0;
            let b = (dec & 0xff) as f64 / 255.0;
            Ok(Color::from_rgba(r, g, b, a))
        } else {
            Err(Error::Hexidecimal("Incorrect hexidecimal string length."))
        }
    }
}

// We manually implement deserialize so we can nicely support letting users use hex or rgba codes.
// Ron says the position is col: 0, line: 0 when we error during this, because we're directly
// deserializing the struct?  Not sure how we would fix this.
impl<'de> Deserialize<'de> for Color {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Intermediate struct with optional fields for ergonomics.
        #[derive(Deserialize)]
        #[serde(rename = "Color")]
        struct Col {
            r: Option<f64>,
            g: Option<f64>,
            b: Option<f64>,
            a: Option<f64>,
            hex: Option<String>,
        }

        // Deserialize into the intermediate struct.
        let col = Col::deserialize(deserializer)?;
        // Check that user hasn't defined both rgba and hex.
        if col.hex.is_some() && (col.r.is_some() || col.g.is_some() || col.b.is_some() || col.a.is_some()) {
            return Err(de::Error::custom(
                "`hex` and `rgba` fields cannot both be present in the same `Color`",
            ));
        }

        if let Some(hex) = col.hex {
            return Color::from_hex(&hex).or(Err(de::Error::invalid_value(
                Unexpected::Str(&hex),
                &"a valid hexidecimal string",
            )));
        } else if let (Some(r), Some(g), Some(b), Some(a)) = (col.r, col.g, col.b, col.a) {
            Ok(Color::from_rgba(r, g, b, a))
        } else {
            Err(de::Error::missing_field("`r`, `g`, `b`, `a` or `hex`"))
        }
    }
}

impl Padding {
    pub fn new(left: f64, right: f64, top: f64, bottom: f64) -> Self {
        Padding {
            left,
            right,
            top,
            bottom,
        }
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
