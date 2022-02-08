use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::sync::atomic::AtomicU32;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use dbus::{
    self,
    MessageType,
    arg::{self, PropMap, RefArg},
    blocking::{Connection, stdintf::org_freedesktop_dbus::RequestNameReply},
    channel::MatchingReceiver,
    message::{MatchRule},
};
use dbus_crossroads::{Crossroads};
use image::{self, DynamicImage, ImageBuffer};

use chrono::{offset::Local, DateTime};
use serde::Serialize;

use crate::bus::dbus_codegen::{self, OrgFreedesktopNotifications};
use crate::maths_utility;
use crate::Config;

static ID_COUNT: AtomicU32 = AtomicU32::new(1);

pub fn fetch_id() -> u32 {
    ID_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

pub const PATH: &str = "/org/freedesktop/Notifications";
// Global access to dbus connection is necessary to avoid spaghetti.
static mut DBUS_CONN: Option<Connection> = None;

pub struct Notify {
    sender: Sender<Message>,
}

impl OrgFreedesktopNotifications for Notify {
    fn get_capabilities(&mut self) -> Result<Vec<String>, dbus::MethodErr> {
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

    fn notify(&mut self, app_name: String, replaces_id: u32, app_icon: String, summary: String, body: String, actions: Vec<String>, hints: arg::PropMap, expire_timeout: i32) -> Result<u32, dbus::MethodErr> {
        // The spec says that:
        // If `replaces_id` is 0, we should create a fresh id and notification.
        // If `replaces_id` is not 0, we should create a replace the notification with that id,
        // using the same id.
        // With our implementation, we send a "new" notification anyway, and let management deal
        // with replacing data.
        // When `Config::replacing_enabled` is `false`, we still obey this, those notifications
        // will just have the same `id`, which I think is fine.
        //
        // @NOTE: Some programs don't seem to obey these rules.  Discord will set replaces_id to `id` no
        // matter what.  To workaround this, we just check if a notification with the same ID
        // exists before sending it (see: `main`), rather than relying on `replaces_id` being set
        // correctly.
        // Also note that there is still a bug here, where since Discord sends the `replaces_id` it
        // is effectively assigning its own id, which may interfere with ours.  Not sure how mmuch I can
        // do about this.
        let id = if replaces_id == 0 {
            // Grab an ID atomically.  This is moreso to allow global access to `ID_COUNT`, but I'm
            // also not sure if `notify` is called in a single-threaded way, so it's best to be safe.
            fetch_id()
        } else {
            replaces_id
        };

        let notification = Notification::from_dbus(
            id,
            app_name,
            app_icon,
            summary,
            body,
            actions,
            hints,
            expire_timeout,
        );

        match self.sender.send(Message::Notify(notification)) {
            Ok(_) => Ok(id),
            Err(e) => Err(dbus::MethodErr::failed(&e)),
        }
    }

    fn close_notification(&mut self, id: u32) -> Result<(), dbus::MethodErr> {
        match self.sender.send(Message::Close(id)) {
            Ok(_) => Ok(()),
            Err(e) => Err(dbus::MethodErr::failed(&e)),
        }
    }

    fn get_server_information(&mut self) -> Result<(String, String, String, String), dbus::MethodErr> {
        Ok((
            env!("CARGO_PKG_NAME").to_string(),
            env!("CARGO_PKG_AUTHORS").to_string(),
            env!("CARGO_PKG_VERSION").to_string(),
            "1.2".to_string(),
        ))
    }
}

pub fn init_dbus_thread() -> (JoinHandle<()>, Receiver<Message>) {
    let (sender, receiver) = mpsc::channel();

    let c = Connection::new_session().expect("Failed to get a session bus.");
    let reply = c
        .request_name("org.freedesktop.Notifications", false, true, false)
        .expect("Failed to register name.");

    // Be helpful to the user.
    match reply {
        RequestNameReply::InQueue => {
            println!("In queue for notification bus name -- is another notification daemon running?")
        }
        _ => {},
        //RequestNameReply::PrimaryOwner => {}, // this happens if there are no other notification daemons.
                                                // we should get the NameAcquired signal shortly.
        //RequestNameReply::Exists => {}  // should never happen, since `do_not_queue` is false.
        //RequestNameReply::AlreadyOwner => {}
    };

    let match_rule = MatchRule::new()
        .with_type(MessageType::Signal)
        .with_interface("org.freedesktop.DBus")
        .with_member("NameAcquired");
    c.add_match(match_rule, |_: (), _conn, msg| {
        if let Some(s) = msg.get1::<&str>() {
            if s == "org.freedesktop.Notifications" {
                println!("Notification bus name acquired.");

                // Stop listening for signals -- name was grabbed.
                return false
            }
        }

        // Keep listening for signals.
        true
    }).expect("Failed to add match.");

    let mut cr = Crossroads::new();
    let token = dbus_codegen::register_org_freedesktop_notifications::<Notify>(&mut cr);
    cr.insert(PATH, &[token], Notify { sender });

    c.start_receive(dbus::message::MatchRule::new_method_call(), Box::new(move |msg, conn| {
        cr.handle_message(msg, conn).unwrap();
        true
    }));

    unsafe {
        DBUS_CONN = Some(c);
    }

    let handle = thread::spawn(process_dbus);
    (handle, receiver)
}

// Check and process any dbus signals.
// Includes senders and receivers.
pub fn process_dbus() {
    let conn = get_connection();
    loop {
        match conn.process(Duration::from_millis(1000)) {
            Ok(_) => (),
            Err(e) => eprintln!("DBus Error: {}", e),
        }
    }
}

pub fn get_connection() -> &'static Connection {
    unsafe {
        assert!(DBUS_CONN.is_some());
        DBUS_CONN.as_ref().unwrap()
    }
}

#[derive(Debug, Clone, Serialize)]
pub enum Urgency {
    Low,
    Normal,
    Critical,
}

impl Default for Urgency {
    fn default() -> Self {
        Self::Normal
    }
}

#[allow(clippy::large_enum_variant)]
pub enum Message {
    Close(u32),
    Notify(Notification),
}

#[derive(Clone, Serialize)]
pub struct Notification {
    pub id: u32,
    pub tag: Option<String>,

    pub app_name: String,

    pub summary: String,
    pub body: String,
    pub actions: HashMap<String, String>,
    #[serde(skip)]
    pub app_image: Option<DynamicImage>,
    #[serde(skip)]
    pub hint_image: Option<DynamicImage>,
    pub percentage: Option<f32>,

    pub urgency: Urgency,

    #[serde(serialize_with="serialize_datetime")]
    pub time: DateTime<Local>,
    pub timeout: i32,
}

use serde::Serializer;

fn serialize_datetime<S>(datetime: &DateTime<Local>, serializer: S) -> Result<S::Ok, S::Error>
where S: Serializer
{
    serializer.serialize_i64(datetime.timestamp())
}

impl std::fmt::Debug for Notification {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Notification: {{\n\tid: {},\n\tapp_name: {},\n\tsummary: {},\n\tbody: {},\n\tactions: {:?},\n\tapp_image: {},\n\thint_image: {},\n\turgency: {:?},\n\tpercentage: {:?},\n\ttime: {},\n\ttimeout: {}\n}}",
            self.id, self.app_name, self.summary, self.body, self.actions, self.app_image.is_some(), self.hint_image.is_some(), self.urgency, self.percentage, self.time, self.timeout,
        )
    }
}

impl Notification {
    pub fn from_self(summary: &str, body: &str, timeout: i32) -> Self {
        let id = fetch_id();
        Self {
            id,
            tag: None,
            app_name: "Wired".to_owned(),
            summary: summary.to_owned(),
            body: body.to_owned(),
            actions: HashMap::new(),
            app_image: None,
            hint_image: None,
            percentage: None,

            urgency: Urgency::Low,

            time: Local::now(),
            timeout,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_dbus(
        id: u32,
        app_name: String,
        app_icon: String,
        summary: String,
        body: String,
        actions: Vec<String>,
        hints: PropMap,
        expire_timeout: i32,
    ) -> Self {
        // The time this notification arrived.  The spec actually doesn't include this for some reason, but
        // we do it for convenience.
        // Put it at the top so it's more accurate to the actual arrival time.
        let time = Local::now();

        // Pango is a bitch about ampersands, and also doesn't decode html entities for us, which
        // applications /love/ to send -- we need to escape ampersands and decode html entities.
        let summary = maths_utility::escape_decode(summary);
        let body = maths_utility::escape_decode(body);

        let mut i = 0;
        let mut actions_map = HashMap::new();
        // The length of this should always be even, since actions are sent as a list of pairs, but
        // we safeguard against bad implementations anyway by checking that i+1 is safe.
        while i < actions.len() {
            actions_map.insert(actions[i].to_owned(), actions[i + 1].to_owned());
            i += 2;
        }

        fn image_from_path(path: &str) -> Option<DynamicImage> {
            //let _start = std::time::Instant::now();
            //dbg!("Loading image from path...");

            // @TODO: this path shouldn't be active if app_icon is empty?
            let img_path = Path::new(path);
            let x = image::open(img_path).ok();

            //let _end = std::time::Instant::now();
            //dbg!(end - start);

            x
        }

        fn image_from_data(data: &VecDeque<Box<dyn RefArg>>) -> Option<DynamicImage> {
            //let start = std::time::Instant::now();
            //dbg!("Loading image from data...");

            let mut it = data.iter();
            let width = *dbus::arg::cast::<i32>(it.next()?)?;
            let height = *dbus::arg::cast::<i32>(it.next()?)?;
            let rowstride = *dbus::arg::cast::<i32>(it.next()?)?;
            let _one_point_two_bit_alpha = *dbus::arg::cast::<bool>(it.next()?)?;
            let bits_per_sample = *dbus::arg::cast::<i32>(it.next()?)?;
            let channels = *dbus::arg::cast::<i32>(it.next()?)?;
            let bytes = dbus::arg::cast::<Vec<u8>>(it.next()?)?.clone();

            // Sometimes dbus (or the application) can give us junk image data, usually when lots of
            // stuff is sent at the same time the same time, so we should sanity check the image.
            // https://github.com/dunst-project/dunst/blob/3f3082efb3724dcd369de78dc94d41190d089acf/src/icon.c#L316
            let pixelstride = (channels * bits_per_sample + 7) / 8;
            let len_expected =
                (height - 1) * rowstride + width * pixelstride;
            let len_actual = bytes.len() as i32;
            if len_actual != len_expected {
                eprintln!(
                    "Expected image data to be of length: {}, but got a length of {}.",
                    len_expected, len_actual,
                );
                return None;
            }

            let x = match channels {
                3 => {
                    ImageBuffer::from_raw(width as u32, height as u32, bytes)
                        .map(DynamicImage::ImageRgb8)
                }
                4 => {
                    ImageBuffer::from_raw(width as u32, height as u32, bytes)
                        .map(DynamicImage::ImageRgba8)
                }
                _ => {
                    eprintln!("Unsupported hint image format!  Couldn't load hint image.");
                    None
                }
            };

            //let end = std::time::Instant::now();
            //dbg!(end - start);

            x
        }

        let app_image = image_from_path(&app_icon);


        // Structs are stored internally in the rust dbus implementation as VecDeque.
        // https://github.com/diwic/dbus-rs/issues/363
        type DBusStruct = VecDeque<Box<dyn RefArg>>;
        // According to the spec, we should do these in this order.
        let hint_image: Option<DynamicImage>;
        if let Some(img_data) = arg::prop_cast::<DBusStruct>(&hints, "image-data") {
            hint_image = image_from_data(img_data);
        } else if let Some(img_data) = arg::prop_cast::<DBusStruct>(&hints, "image_data") {
            hint_image = image_from_data(img_data);
        } else if let Some(img_path) = hints.get("image-path") {
            hint_image = image_from_path(img_path.as_str().unwrap());
        } else if let Some(img_path) = hints.get("image_path") {
            // TODO: fix ugly.
            hint_image = image_from_path(img_path.as_str().unwrap());
        } else if let Some(img_data) = arg::prop_cast::<DBusStruct>(&hints, "icon_data") {
            hint_image = image_from_data(img_data);
        } else {
            hint_image = None;
        }

        let urgency: Urgency;
        if let Some(level) = arg::prop_cast::<u8>(&hints, "urgency") {
            match *level {
                0 => urgency = Urgency::Low,
                1 => urgency = Urgency::Normal,
                2 => urgency = Urgency::Critical,
                _ => urgency = Urgency::Normal,
            }
        } else {
            urgency = Urgency::Normal;
        }

        let tag = arg::prop_cast::<String>(&hints, "wired-tag").cloned();

        let percentage: Option<f32>;
        if let Some(value) = arg::prop_cast::<i32>(&hints, "value") {
            // This should be ok since we only support values from 0 to 100.
            let v = *value as f32;
            let p = f32::clamp(v * 0.01, 0.0, 1.0);
            percentage = Some(p)
        } else {
            percentage = None;
        }

        let mut timeout = expire_timeout;
        if timeout <= 0 {
            timeout = Config::get().timeout;
        }

        Self {
            id,
            tag,
            app_name: app_name.to_owned(),
            summary,
            body,
            actions: actions_map,
            app_image,
            hint_image,
            urgency,
            percentage,
            time,
            timeout,
        }
    }

    pub fn get_default_action(&self) -> Option<(String, String)> {
        self.actions
            .get_key_value("default")
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
    }

    pub fn get_other_action(&self, idx: usize) -> Option<(String, String)> {
        // Creates an iterator without the "default" key, which is preserved for action1.
        let mut keys = self.actions.keys().filter(|s| *s != "default");
        let maybe_key = keys.nth(idx);
        if let Some(key) = maybe_key {
            self.actions
                .get_key_value(key)
                .map(|(k, v)| (k.to_owned(), v.to_owned()))
        } else {
            None
        }
    }
}
