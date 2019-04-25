use std::collections::HashMap;
use std::sync::mpsc::Sender;

use dbus::arg;

use super::dbus_codegen::{ OrgFreedesktopNotifications };
use super::dbus::Notification;

#[derive(Copy, Clone, Default, Debug)]
pub struct BusNotification;
impl OrgFreedesktopNotifications for BusNotification {
    type Err = dbus::tree::MethodErr;
    fn close_notification(&self, _id: u32) -> Result<(), Self::Err> {
        Ok(())
    }

    fn get_capabilities(&self) -> Result<Vec<String>, Self::Err> {
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
    fn get_server_information(&self) -> Result<(String, String, String, String), Self::Err> {
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
        _hints: HashMap<&str, arg::Variant<Box<arg::RefArg>>>,
        expire_timeout: i32,
        ) -> Result<u32, Self::Err> {
        let notification = Notification::new(
            app_name.to_owned(),
            replaces_id,
            app_icon.to_owned(),
            summary.to_owned(),
            body.to_owned(),
            expire_timeout
        );

        sender.send(notification).unwrap();

        Ok(0 as u32)
    }
}
