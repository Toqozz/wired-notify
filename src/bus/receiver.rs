use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;

use dbus::tree;
use crate::bus::dbus::Notification;

use super::dbus_codegen::{ OrgFreedesktopNotifications, Value };

static ID_COUNT: AtomicU32 = AtomicU32::new(1);

#[derive(Copy, Clone, Default, Debug)]
pub struct BusNotification;
impl OrgFreedesktopNotifications for BusNotification {
    fn close_notification(&self, _id: u32) -> Result<(), tree::MethodErr> {
        Ok(())
    }

    fn get_capabilities(&self) -> Result<Vec<String>, tree::MethodErr> {
        let capabilities: Vec<String> = vec![
            //"action-icons".to_string(),
            "actions".to_string(),
            "body".to_string(),
            "body-hyperlinks".to_string(),
            "body-markup".to_string(),
            //"icon-multi".to_string(),
            "icon-static".to_string(),
            //"persistence".to_string(),
            //"sound".to_string(),
        ];

        Ok(capabilities)
    }

    fn get_server_information(&self) -> Result<(String, String, String, String), tree::MethodErr> {
        Ok((
            env!("CARGO_PKG_NAME").to_string(),
            env!("CARGO_PKG_AUTHORS").to_string(),
            env!("CARGO_PKG_VERSION").to_string(),
            "1.2".to_string(),
        ))
    }

    fn notify(
        &self,
        sender: Sender<Notification>,
        app_name: &str,
        replaces_id: u32,
        app_icon: &str,
        summary: &str,
        body: &str,
        actions: Vec<&str>,
        hints: HashMap<String, Value>,
        expire_timeout: i32,
        ) -> Result<u32, tree::MethodErr> {

        // The spec says that:
        // If `replaces_id` is 0, we should create a fresh id and notification.
        // If `replaces_id` is not 0, we should create a replace the notification with that id,
        // using the same id.
        // With our implementation, we send a "new" notification anyway, and let management deal
        // with replacing data.
        let id = if replaces_id == 0 {
            // Grab an ID atomically.  This is moreso to allow global access to `ID_COUNT`, but I'm
            // also not sure if `notify` is called in a single-threaded way, so it's best to be safe.
            ID_COUNT.fetch_add(1, Ordering::Relaxed)
        } else {
            replaces_id
        };

        let notification = Notification::from_dbus(
            id, app_name, replaces_id, app_icon, summary, body, actions, hints, expire_timeout,
        );

        sender.send(notification).unwrap();

        Ok(id)
    }
}
