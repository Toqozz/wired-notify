use std::sync::Arc;
use std::sync::mpsc::{self, Receiver};

use dbus::{
    self,
    tree::{self, DataType, Interface, Factory, Tree},
    ffidisp::{Connection, BusType, NameFlag, RequestNameReply},
};

use crate::bus::receiver::BusNotification;
use crate::bus::dbus_codegen::org_freedesktop_notifications_server;

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

fn create_iface(sender: mpsc::Sender<DBusNotification>) -> Interface<tree::MTFn<TData>, TData> {
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

pub fn init_bus(sender: mpsc::Sender<DBusNotification>) -> Connection {
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

pub fn get_connection() -> (Connection, Receiver<DBusNotification>) {
    let (sender, receiver) = mpsc::channel();
    let c = init_bus(sender);
    (c, receiver)
}

#[derive(Debug)]
pub struct DBusNotification {
    // Notification info.
    pub app_name: String,
    pub replaces_id: u32,
    pub app_icon: String,
    pub summary: String,
    pub body: String,
    pub expire_timeout: i32,
}

impl DBusNotification {
    pub fn new(
        app_name: String,
        replaces_id: u32,
        app_icon: String,
        summary: String,
        body: String,
        expire_timeout: i32
        ) -> Self {

        Self { app_name, replaces_id, app_icon, summary, body, expire_timeout }
    }
}
