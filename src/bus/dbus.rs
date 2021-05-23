use std::sync::Arc;
use std::sync::mpsc::{self, Receiver};
use std::collections::HashMap;
use std::path::Path;

use image::{self, DynamicImage, ImageBuffer};
use dbus::{
    self,
    tree::{self, DataType, Interface, Factory, Tree},
    ffidisp::{Connection, BusType, NameFlag, RequestNameReply},
};

use chrono::{ DateTime, Utc };

use crate::Config;
use crate::bus::receiver::BusNotification;
use crate::bus::dbus_codegen::{org_freedesktop_notifications_server, Value, DBusImage};
use crate::maths_utility;

#[derive(Copy, Clone, Default, Debug)]
struct TData;
impl DataType for TData {
    type Tree = ();
    type ObjectPath = Arc<BusNotification>;
    type Property = ();
    type Interface = ();
    type Method = ();
    type Signal = ();
}

pub const PATH: &str = "/org/freedesktop/Notifications";

fn create_iface(sender: mpsc::Sender<Message>) -> Interface<tree::MTFn<TData>, TData> {
    let f = Factory::new_fn();
    org_freedesktop_notifications_server(sender, &f, (), |m| {
        let a: &Arc<BusNotification> = m.path.get_data();
        let b: &BusNotification = &a;
        b
    })
}

fn create_tree(iface: Interface<tree::MTFn<TData>, TData>) -> Tree<tree::MTFn<TData>, TData> {
    let n = Arc::new(BusNotification);

    let f = Factory::new_fn();
    let mut tree = f.tree(());
    tree = tree.add(f.object_path(PATH, n)
        .introspectable()
        .add(iface));

    tree
}

pub fn init_bus(sender: mpsc::Sender<Message>) -> Connection {
    let iface = create_iface(sender);
    let tree = create_tree(iface);

    let c = Connection::get_private(BusType::Session).expect("Failed to get a session bus.");
    let reply = c.register_name("org.freedesktop.Notifications", NameFlag::ReplaceExisting as u32)
        .expect("Failed to register name.");

    // Be helpful to the user.
    match reply {
        RequestNameReply::PrimaryOwner => println!("Acquired notification bus name."),
        RequestNameReply::InQueue => println!("In queue for notification bus name -- is another notification daemon running?"),
        RequestNameReply::Exists => {},
        RequestNameReply::AlreadyOwner => {},
    };

    tree.set_registered(&c, true).unwrap();

    c.add_handler(tree);
    c
}

pub fn get_connection() -> (Connection, Receiver<Message>) {
    let (sender, receiver) = mpsc::channel();
    let c = init_bus(sender);
    (c, receiver)
}

#[derive(Debug)]
pub enum Urgency {
    Low,
    Normal,
    Critical,
}

impl Default for Urgency {
    fn default() -> Self { Self::Normal }
}

pub enum Message {
    Close(u32),
    Notify(Notification),
}

pub struct Notification {
    pub id: u32,

    pub app_name: String,
    pub replaces_id: u32,

    pub summary: String,
    pub body: String,
    pub actions: HashMap<String, String>,
    pub app_image: Option<DynamicImage>,
    pub hint_image: Option<DynamicImage>,

    pub urgency: Urgency,

    pub time: DateTime<Utc>,
    pub timeout: i32,
}

impl std::fmt::Debug for Notification {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Notification: {{\n\tid: {},\n\tapp_name: {},\n\treplaces_id: {},\n\tsummary: {},\n\tbody: {},\n\tactions: {:?},\n\tapp_image: {},\n\thint_image: {},\n\ttimeout: {}\n}}",
            self.id, self.app_name, self.replaces_id, self.summary, self.body, self.actions, self.app_image.is_some(), self.hint_image.is_some(), self.timeout,
        )
    }
}

impl Notification {
    pub fn from_dbus(
        id: u32,
        app_name: &str,
        replaces_id: u32,
        app_icon: &str,
        summary: &str,
        body: &str,
        actions: Vec<&str>,
        mut hints: HashMap<String, Value>,
        expire_timeout: i32,
    ) -> Self {
        // The time this notification arrived.  The spec actually doesn't include this for some reason, but
        // we do it for convenience.
        // Put it at the top so it's more accurate to the actual arrival time.
        let time = Utc::now();

        // Pango is a bitch about ampersands, and also doesn't decode html entities for us, which
        // applications /love/ to send -- we need to escape ampersands and decode html entities.
        let summary = maths_utility::escape_decode(summary);
        let body = maths_utility::escape_decode(body);

        let mut i = 0;
        let mut actions_map = HashMap::new();
        // The length of this should always be even, since actions are sent as a list of pairs, but
        // we safeguard against bad implementations anyway by checking that i+1 is safe.
        while i < actions.len() {
            actions_map.insert(actions[i].to_owned(), actions[i+1].to_owned());
            i += 2;
        }

        fn image_from_path(path: &str) -> Option<DynamicImage> {
            let _start = std::time::Instant::now();
            //dbg!("Loading image from path...");

            // @TODO: this path shouldn't be active if app_icon is empty?
            let img_path = Path::new(path);
            let x = image::open(img_path).ok();

            let _end = std::time::Instant::now();
            //dbg!(end - start);

            return x;
        }

        fn image_from_data(dbus_image: DBusImage) -> Option<DynamicImage> {
            //let start = std::time::Instant::now();
            //dbg!("Loading image from data...");

            // Sometimes dbus (or the application) can give us junk image data, usually when lots of
            // stuff is sent at the same time the same time, so we should sanity check the image.
            // https://github.com/dunst-project/dunst/blob/3f3082efb3724dcd369de78dc94d41190d089acf/src/icon.c#L316
            let pixelstride = (dbus_image.channels * dbus_image.bits_per_sample + 7)/8;
            let len_expected = (dbus_image.height - 1) * dbus_image.rowstride + dbus_image.width * pixelstride;
            let len_actual = dbus_image.data.len() as i32;
            if len_actual != len_expected {
                eprintln!(
                    "Expected image data to be of length: {}, but got a length of {}.",
                    len_expected, len_actual,
                );
                return None;
            }

            let x = match dbus_image.channels {
                3 => ImageBuffer::from_raw(dbus_image.width as u32, dbus_image.height as u32, dbus_image.data)
                        .map(DynamicImage::ImageRgb8),
                4 => ImageBuffer::from_raw(dbus_image.width as u32, dbus_image.height as u32, dbus_image.data)
                        .map(DynamicImage::ImageRgba8),
                _ => {
                    eprintln!("Unsupported hint image format!  Couldn't load hint image.");
                    None
                },
            };

            //let end = std::time::Instant::now();
            //dbg!(end - start);

            return x;
        }

        let app_image = image_from_path(&app_icon);

        let hint_image: Option<DynamicImage>;
        // We want to pass the `dbus_image.data` vec rather than cloning it, so we have to remove it
        // from the array.
        // An alternative might be to put `data` in an option or something like that.
        if let Some(Value::Struct(dbus_image)) = hints.remove("image-data").or(hints.remove("image_data")) {
            hint_image = image_from_data(dbus_image);
        } else if let Some(Value::String(path)) = hints.get("image-path").or(hints.get("image_path")) {
            hint_image = image_from_path(&path);
        } else if let Some(Value::Struct(dbus_image)) = hints.remove("icon_data") {
            hint_image = image_from_data(dbus_image);
        } else {
            hint_image = None;
        }

        let urgency: Urgency;
        if let Some(Value::U8(level)) = hints.get("urgency") {
            match level {
                0 => urgency = Urgency::Low,
                1 => urgency = Urgency::Normal,
                2 => urgency = Urgency::Critical,
                _ => urgency = Urgency::Normal,
            }
        } else {
            urgency = Urgency::Normal;
        }

        let mut timeout = expire_timeout;
        if timeout <= 0 {
            timeout = Config::get().timeout;
        }

        Self {
            id,
            app_name: app_name.to_owned(),
            replaces_id,
            summary,
            body,
            actions: actions_map,
            app_image,
            hint_image,
            urgency,
            time,
            timeout,
        }
    }
}
