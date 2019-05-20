use std::sync::Arc;
use std::sync::mpsc;

use dbus;
use dbus::tree::{ self, DataType, Interface, Factory, Tree };

use super::receiver::BusNotification;
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

pub fn init_bus(sender: mpsc::Sender<Notification>) -> dbus::Connection {
    //let f = Factory::new_fn::<()>();
    let iface = create_iface(sender);
    let tree = create_tree(iface);

    let c = dbus::Connection::get_private(dbus::BusType::Session).expect("Failed to get a session bus.");
    c.register_name("org.freedesktop.Notifications", dbus::NameFlag::ReplaceExisting as u32).expect("Failed to register name.");
    tree.set_registered(&c, true).unwrap();

    c.add_handler(tree);
    c
}

pub fn dbus_loop(sender: mpsc::Sender<Notification>) -> dbus::Connection {
    let c = init_bus(sender);
    c

    /*
    loop {
        let signal = c.incoming(500).next();
        if let Some(s) = signal {
            dbg!(s);
        }

        if let Ok(x) = receiver.try_recv() {
            dbg!(x);
        }
    }
    */
}

#[derive(Debug)]
pub struct Notification {
    // Notification info.
    pub app_name: String,
    pub replaces_id: u32,
    pub app_icon: String,
    pub summary: String,
    pub body: String,
    pub expire_timeout: i32,
}

impl Notification {
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
