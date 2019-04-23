use std::collections::HashMap;

use dbus::arg;

mod crate::dbus_codegen;
use dbus_codegen::{ OrgFreedesktopNotifications };

#[derive(Copy, Clone, Default, Debug)]
struct Notification<'a> {
    app_name: &'a str,
    app_icon: &'a str,
    summary:  &'a str,
    body:     &'a str,
    replaces_id:    u32,
    expire_timeout: u32,
}

impl<'a> OrgFreedesktopNotifications for Notification<'a> {
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
        app_name: &str,
        replaces_id: u32,
        app_icon: &str,
        summary: &str,
        body: &str,
        _actions: Vec<&str>,
        _hints: HashMap<&str, arg::Variant<Box<arg::RefArg>>>,
        expire_timeout: i32,
        ) -> Result<u32, Self::Err> {
        println!(
            "notification: app_name={}, replaces_id={}, app_icon={}, summary={}, body={}",
            app_name, replaces_id, app_icon, summary, body
        );

        Ok(0 as u32)
    }
}

struct Message<'a> {
    summary: &'a str,
    body: &'a str,
}
