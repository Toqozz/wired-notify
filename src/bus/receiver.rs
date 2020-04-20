use std::collections::HashMap;
use std::sync::mpsc::Sender;

use dbus::tree;
use crate::bus::dbus::Notification;

use super::dbus_codegen::{ OrgFreedesktopNotifications, Value };

#[derive(Copy, Clone, Default, Debug)]
pub struct BusNotification;
impl OrgFreedesktopNotifications for BusNotification {
    //type Err = dbus::tree::MethodErr;
    fn close_notification(&self, _id: u32) -> Result<(), tree::MethodErr> {
        Ok(())
    }

    fn get_capabilities(&self) -> Result<Vec<String>, tree::MethodErr> {
        let capabilities: Vec<String> = vec![
            "actions".to_string(),
            "body".to_string(),
            "body-hyperlinks".to_string(),
            "body-markup".to_string(),
            "icon-static".to_string(),
            "sound".to_string(),
            "persistence".to_string(),
            "action-icons".to_string(),
        ];

        Ok(capabilities)
    }

    // TODO: fill out this.
    fn get_server_information(&self) -> Result<(String, String, String, String), tree::MethodErr> {
        Ok((
            "dummy".to_string(),
            "dummy".to_string(),
            "dummy".to_string(),
            "dummy".to_string(),
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
        _actions: Vec<&str>,
        hints: HashMap<String, Value>,
        expire_timeout: i32,
        ) -> Result<u32, tree::MethodErr> {

        let notification = Notification::from_dbus(
            app_name, replaces_id, app_icon, summary, body, hints, expire_timeout,
        );

        sender.send(notification).unwrap();

        Ok(0 as u32)
    }
}
