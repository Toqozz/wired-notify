pub mod management;

use image::{self, DynamicImage};

use std::fmt;
use std::path::Path;

use crate::bus::dbus::DBusNotification;
use crate::config::Config;

pub struct Notification {
    pub summary: String,
    pub body: String,
    pub image: Option<DynamicImage>,

    pub timeout: i32,
}

impl fmt::Debug for Notification {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Notification: {{ summary: {}, body: {}, image: {}, timeout: {}",
            self.summary, self.body, self.image.is_some(), self.timeout,
        )
    }
}

impl Notification {
    pub fn from_dbus(notification: DBusNotification) -> Self {
        let mut timeout = notification.expire_timeout;
        if timeout <= 0 {
            timeout = Config::get().timeout;
        }

        dbg!(&notification);
        let img_path = Path::new(&notification.app_icon);
        let image = image::open(img_path).ok();

        Notification {
            summary: notification.summary,
            body: notification.body,

            image,
            timeout,
        }
    }
}
