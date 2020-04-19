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

use crate::Config;
use crate::bus::receiver::BusNotification;
use crate::bus::dbus_codegen::{org_freedesktop_notifications_server, Value, DBusImage};

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

fn create_iface(sender: mpsc::Sender<Notification>) -> Interface<tree::MTFn<TData>, TData> {
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
    tree = tree.add(f.object_path("/org/freedesktop/Notifications", n)
        .introspectable()
        .add(iface));

    tree
}

pub fn init_bus(sender: mpsc::Sender<Notification>) -> Connection {
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

pub fn get_connection() -> (Connection, Receiver<Notification>) {
    let (sender, receiver) = mpsc::channel();
    let c = init_bus(sender);
    (c, receiver)
}

pub struct Notification {
    pub app_name: String,
    pub replaces_id: u32,

    pub summary: String,
    pub body: String,
    pub app_image: Option<DynamicImage>,
    pub hint_image: Option<DynamicImage>,

    pub timeout: i32,
}

impl std::fmt::Debug for Notification {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Notification: {{\n\tapp_name: {}, replaces_id: {}, summary: {}, body: {}, app_image: {}, hint_image: {}, timeout: {}\n}}",
            self.app_name, self.replaces_id, self.summary, self.body, self.app_image.is_some(), self.hint_image.is_some(), self.timeout,
        )
    }
}

fn escape_ampersand(to_escape: &str) -> String {
    // Escape ampersand manually, for fun.
    // can escape 3 ampersands without allocating (each is 4 chars, minus the existing char).
    let mut escaped: Vec<u8> = Vec::with_capacity(to_escape.len() + 9);
    let bytes = to_escape.as_bytes();
    for i in 0..bytes.len() {
        let byte = bytes[i];
        match byte {
            b'&' => {
                if i + 5 <= to_escape.len() {
                    match &to_escape[i..i+5] {
                        "&amp;" => escaped.push(byte),
                        _ => escaped.extend_from_slice(b"&amp;"),
                    }
                } else {
                    escaped.extend_from_slice(b"&amp;");
                }
            }
            _ => escaped.push(byte),
        }
    }

    // We should be safe to use `from_utf8_unchecked` here, but let's be safe.
    String::from_utf8(escaped).expect("Error when escaping ampersand.")
}

impl Notification {
    pub fn from_dbus(
        app_name: &str,
        replaces_id: u32,
        app_icon: &str,
        summary: &str,
        body: &str,
        mut hints: HashMap<String, Value>,
        expire_timeout: i32,
    ) -> Self {

        // Pango is a bitch about ampersands -- we need to escape them.
        let summary = escape_ampersand(summary);
        let body = escape_ampersand(body);

        fn image_from_path(path: &str) -> Option<DynamicImage> {
            // @TODO: this path shouldn't be active if app_icon is empty?
            let img_path = Path::new(path);
            image::open(img_path).ok()
        }

        fn image_from_data(dbus_image: DBusImage) -> Option<DynamicImage> {
            match dbus_image.channels {
                3 => ImageBuffer::from_raw(dbus_image.width as u32, dbus_image.height as u32, dbus_image.data)
                        .map(DynamicImage::ImageRgb8),
                4 => ImageBuffer::from_raw(dbus_image.width as u32, dbus_image.height as u32, dbus_image.data)
                        .map(DynamicImage::ImageRgba8),
                _ => {
                    eprintln!("Unsupported hint image format!  Couldn't load hint image.");
                    None
                },
            }
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

        let mut timeout = expire_timeout;
        if timeout <= 0 {
            timeout = Config::get().timeout;
        }

        Self {
            app_name: app_name.to_owned(),
            replaces_id,
            summary,
            body,
            app_image,
            hint_image,
            timeout,
        }
    }
}
