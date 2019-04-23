use std::sync::Arc;
use std::sync::mpsc::{ Sender, Receiver };

use dbus;
use dbus::tree;
use dbus::tree::{ DataType, Interface, Factory, Tree };

use super::message::BusNotification;
use super::dbus_codegen::org_freedesktop_notifications_server;

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

fn create_iface(sender: Sender<Notification>) -> Interface<tree::MTFn<TData>, TData> {
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

pub fn init_bus(sender: Sender<Notification>) -> dbus::Connection {
    let f = Factory::new_fn::<()>();
    let iface = create_iface(sender);
    let tree = create_tree(iface);

    let c = dbus::Connection::get_private(dbus::BusType::Session).expect("Failed to get a session bus.");
    c.register_name("org.freedesktop.Notifications", dbus::NameFlag::ReplaceExisting as u32).expect("Failed to register name.");
    tree.set_registered(&c, true).unwrap();

    c.add_handler(tree);
    c
}

//static mut DBUS_SENDER: Option<Sender<Notification>> = None;

#[derive(Debug)]
pub struct Notification {
    app_name: String,
    replaces_id: u32,
    app_icon: String,
    summary: String,
    body: String,
    expire_timeout: i32,
}

impl Notification {
    pub fn new(app_name: String, replaces_id: u32, app_icon: String, summary: String, body: String, expire_timeout: i32) -> Notification {
        Notification { app_name, replaces_id, app_icon, summary, body, expire_timeout }
    }
}

pub fn dbus_loop(sender: Sender<Notification>, receiver: Receiver<Notification>) {
    let c = init_bus(sender);

    loop {
        // TODO: can get found name messages here.
        c.incoming(500).next();

        if let Ok(x) = receiver.try_recv() {
            dbg!(x);
        }
    }
}
