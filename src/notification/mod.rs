pub mod management;

use image::{self, DynamicImage};

use std::fmt;
use std::path::Path;

use dbus::arg::{self, RefArg};
//use crate::bus::dbus::DBusNotification;
use crate::config::Config;

/*
pub struct Notification {
    pub summary: String,
    pub body: String,
    pub app_image: Option<DynamicImage>,
    //pub hint_image: Option<DynamicImage>,

    pub timeout: i32,
}

impl fmt::Debug for Notification {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Notification: {{ summary: {}, body: {}, app_image: {}, timeout: {}",
            self.summary, self.body, self.app_image.is_some(), self.timeout,
        )
    }
}

impl Notification {
    pub fn from_dbus(notification: DBusNotification) -> Self {
        let mut timeout = notification.expire_timeout;
        if timeout <= 0 {
            timeout = Config::get().timeout;
        }

        // @TODO: this path shouldn't be active if app_icon is empty?
        let img_path = Path::new(&notification.app_icon);
        let app_image = image::open(img_path).ok();

        /*
        fn get_image_data(image_hint: &arg::Variant<Box<dyn arg::RefArg>>) -> DBusImage {
            let mut it = image_hint.0.as_iter().unwrap();

            let width = it.next().unwrap().as_i64().unwrap() as i32;
            let height = it.next().unwrap().as_i64().unwrap() as i32;
            let rowstride = it.next().unwrap().as_i64().unwrap() as i32;
            let one_point_two_bit_alpha = it.next().unwrap().as_i64().unwrap() != 0;
            let bits_per_sample = it.next().unwrap().as_i64().unwrap() as i32;
            let channels = it.next().unwrap().as_i64().unwrap() as i32;

            let data_c = &it.next().unwrap().as_any();
            //let data_c = &it.next().unwrap().box_clone();
            //dbg!(&**data_c);
            //let data = arg::cast::<Vec<u8>>(&**data_c).expect("wtf");
            //dbg!(data_c);

            let img = DBusImage {
                width,
                height,
                rowstride,
                one_point_two_bit_alpha,
                bits_per_sample,
                channels,
                data: vec![],
            };

            dbg!(&img);
            img
        }
        */

        /*
        let mut dbus_image = None;
        if let Some(image_hint) = notification.hints.get("icon_data") {
            dbus_image = Some(get_image_data(image_hint));
        }


        let hint_image =
            if let Some(d_img) = dbus_image {
                Some(
                    DynamicImage::ImageRgb8(
                        image::ImageBuffer::from_raw(
                            d_img.width as u32,
                            d_img.height as u32,
                            d_img.data
                        ).unwrap()
                    )
                )
            } else {
                None
            };
        */

        Notification {
            summary: notification.summary,
            body: notification.body,

            app_image,
            //hint_image,
            timeout,
        }
    }
}
*/
