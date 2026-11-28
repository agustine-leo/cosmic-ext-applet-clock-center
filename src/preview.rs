// SPDX-License-Identifier: GPL-3.0-only
//! Dev-only window that draws the popup with sample data, so the design can
//! be captured without clicking the panel: `CLOCK_CENTER_PREVIEW=1
//! cosmic-ext-applet-clock-center`. `CLOCK_CENTER_PREVIEW=empty` shows the
//! empty states (no notifications, no player, weather not yet fetched);
//! `expanded` opens a row and a side; `settings` opens the world clock and
//! weather settings with sample search results. All content is synthetic.

use std::collections::HashSet;
use std::time::{Duration, Instant, SystemTime};

use cosmic::app::{Core, Settings, Task};
use cosmic::iced::{Length, Size};
use cosmic::widget::{container, icon};
use cosmic::{executor, Element};
use cosmic_notifications_util::{ActionId, Hint, Image, Notification};

use crate::app::{popup_content, Message, PopupData, POPUP_SIZE, PREVIEW_PADDING};
use crate::media::{PlaybackStatus, PlayerState};
use crate::notifications::{Expansion, NotificationStore};

pub fn run(mode: &str) -> cosmic::iced::Result {
    let settings = Settings::default().size(Size::new(POPUP_SIZE.0, POPUP_SIZE.1)).resizable(None);
    cosmic::app::run::<Preview>(settings, mode.to_string())
}

pub struct Preview {
    core: Core,
    store: NotificationStore,
    players: Vec<PlayerState>,
    art: icon::Handle,
    cards: HashSet<u32>,
    groups: HashSet<String>,
    clocks: Vec<crate::config::WorldClock>,
    places: Vec<crate::config::WeatherPlace>,
    readings: std::collections::HashMap<String, crate::extras::Weather>,
    now: jiff::Zoned,
    edit: Option<crate::extras::Edit>,
}

#[allow(clippy::too_many_arguments)]
fn sample(id: u32, app: &str, icon: &str, summary: &str, body: &str, urgency: u8, mins_ago: u64, actions: &[(&str, &str)]) -> Notification {
    Notification {
        id,
        app_name: app.into(),
        app_icon: icon.into(),
        summary: summary.into(),
        body: body.into(),
        actions: actions.iter().map(|(a, l)| (a.parse::<ActionId>().unwrap(), l.to_string())).collect(),
        hints: vec![Hint::Urgency(urgency)],
        expire_timeout: -1,
        time: SystemTime::now() - Duration::from_secs(mins_ago * 60),
    }
}

/// Synthetic RGBA square: a three-stop diagonal gradient (album art / avatar stand-in).
fn raster(size: u32, a: [u8; 3], b: [u8; 3], c: [u8; 3]) -> Vec<u8> {
    let mut v = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let t = (x + y) as f32 / (2 * size) as f32;
            let (p, q, k) = if t < 0.5 { (a, b, t * 2.0) } else { (b, c, (t - 0.5) * 2.0) };
            for i in 0..3 {
                v.push((p[i] as f32 + (q[i] as f32 - p[i] as f32) * k) as u8);
            }
            v.push(255);
        }
    }
    v
}

impl cosmic::Application for Preview {
    type Executor = executor::Default;
    type Flags = String;
    type Message = Message;
    const APP_ID: &'static str = "io.github.agustine_leo.CosmicExtAppletClockCenterPreview";

    fn core(&self) -> &Core {
        &self.core
    }
    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, mode: String) -> (Self, Task<Message>) {
        let empty = mode == "empty";
        let mut store = NotificationStore::default();
        let mut players = Vec::new();
        if !empty {
            let mut mum = sample(1, "Whatsie", "whatsie", "Mum", "Dinner at 7? Bring the dish you borrowed last week if you still have it.", 1, 0, &[("reply", "Reply"), ("read", "Mark as Read")]);
            mum.hints.push(Hint::Image(Image::Data { width: 96, height: 96, data: raster(96, [0x4f, 0x6f, 0x52], [0x7d, 0x9a, 0x6e], [0xd9, 0xe4, 0xd2]) }));
            for n in [
                sample(5, "Files", "folder-download", "Download complete", "fedora-workstation-45.iso", 0, 160, &[("open", "Open")]),
                sample(4, "Whatsie", "whatsie", "Dad", "Call me when you can", 1, 92, &[]),
                sample(3, "Battery", "battery-caution-symbolic", "Battery at 6%", "Plug in now. The laptop suspends in about 4 minutes.", 2, 3, &[]),
                sample(2, "Zen Browser", "zen", "Standup in 5 minutes", "meet.google.com · Platform team", 1, 2, &[]),
                mum,
            ] {
                store.push(n);
            }
            players.push(PlayerState {
                bus_name: "org.mpris.MediaPlayer2.spotify".into(),
                identity: "Spotify".into(),
                title: "Plastic Love".into(),
                artist: "Mariya Takeuchi".into(),
                art_url: None,
                status: PlaybackStatus::Playing,
                last_change: Instant::now(),
            });
        }
        let art = icon::from_raster_pixels(256, 256, raster(256, [0xd7, 0x7a, 0x8f], [0xf2, 0xc4, 0x6d], [0x3b, 0x6e, 0xa8]));
        let clocks = vec![
            crate::config::WorldClock { label: "Tokyo".into(), zone: "Asia/Tokyo".into() },
            crate::config::WorldClock { label: "London".into(), zone: "Europe/London".into() },
        ];
        let places = vec![
            crate::config::WeatherPlace { name: "Kuala Lumpur".into(), latitude: 3.14, longitude: 101.69 },
            crate::config::WeatherPlace { name: "Kota Kinabalu".into(), latitude: 5.98, longitude: 116.07 },
        ];
        // Sample readings (synthetic), so the preview needs no network.
        let mut readings = std::collections::HashMap::new();
        if !empty {
            readings.insert(places[0].key(), crate::extras::Weather { temperature: 31.0, code: 95, high: 33.0, low: 25.0, is_day: true });
            readings.insert(places[1].key(), crate::extras::Weather { temperature: 29.0, code: 2, high: 31.0, low: 24.0, is_day: true });
        }
        (
            Preview {
                core,
                store,
                players,
                art,
                // `expanded`: the newest chat row open and its side showing all rows.
                cards: if mode == "expanded" { HashSet::from([1]) } else { HashSet::new() },
                groups: if mode == "expanded" { HashSet::from(["Whatsie".to_string()]) } else { HashSet::new() },
                clocks,
                places,
                readings,
                now: jiff::Zoned::now(),
                // `settings`: the credits in edit mode with sample search results.
                edit: (mode == "settings").then(|| crate::extras::Edit {
                    target: Some(crate::extras::Target::Clock),
                    query: "Tok".into(),
                    generation: 0,
                    search: crate::extras::Search::Found(vec![
                        crate::extras::City { name: "Tokyo".into(), detail: "Japan".into(), latitude: 35.69, longitude: 139.69, timezone: Some("Asia/Tokyo".into()) },
                        crate::extras::City { name: "Tokoroa".into(), detail: "Waikato, New Zealand".into(), latitude: -38.22, longitude: 175.87, timezone: Some("Pacific/Auckland".into()) },
                    ]),
                }),
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ToggleNotificationExpanded(id) => {
                if !self.cards.remove(&id) {
                    self.cards.insert(id);
                }
            }
            Message::ToggleNotificationGroup(app) => {
                if !self.groups.remove(&app) {
                    self.groups.insert(app);
                }
            }
            Message::DismissNotification(id) => self.store.dismiss(id),
            Message::ClearNotifications => self.store.clear(),
            Message::EditExtras(open) => self.edit = open.then(Default::default),
            Message::CitySearchInput(q) => {
                if let Some(e) = self.edit.as_mut() {
                    e.query = q;
                }
            }
            other => println!("preview: {other:?}"),
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let today = jiff::Zoned::now().date();
        container(popup_content(PopupData {
            players: &self.players,
            art: Some(&self.art),
            store: &self.store,
            status: None,
            expansion: Expansion { cards: &self.cards, groups: &self.groups },
            do_not_disturb: false,
            shown: today,
            today,
            first_day_of_week: 6,
            clocks: &self.clocks,
            places: &self.places,
            readings: &self.readings,
            now: &self.now,
            edit: self.edit.as_ref(),
        }))
        .padding(PREVIEW_PADDING)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}
