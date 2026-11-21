// SPDX-License-Identifier: GPL-3.0-only

//! Media controls for MPRIS players on the session bus.

use std::{collections::HashMap, path::PathBuf, time::Instant};

use cosmic::{
    iced::{
        advanced::text::{Ellipsize, EllipsizeHeightLimit},
        futures, stream, Alignment, Length, Subscription,
    },
    widget::{button, column, container, icon, row, text},
    Element,
};
use futures_util::{SinkExt, StreamExt};
use zbus::{
    fdo::DBusProxy,
    message::Type as MessageType,
    proxy,
    proxy::CacheProperties,
    zvariant::{OwnedValue, Value},
    Connection, MatchRule, MessageStream,
};

use crate::app::Message;

const MPRIS_PREFIX: &str = "org.mpris.MediaPlayer2.";
const MPRIS_PATH: &str = "/org/mpris/MediaPlayer2";
/// playerctld re-exports other players; showing it would duplicate them.
const IGNORED: &[&str] = &["org.mpris.MediaPlayer2.playerctld"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackStatus {
    Playing,
    Paused,
    Stopped,
}

impl PlaybackStatus {
    fn parse(s: &str) -> Self {
        match s {
            "Playing" => Self::Playing,
            "Paused" => Self::Paused,
            _ => Self::Stopped,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerState {
    pub bus_name: String,
    pub identity: String,
    pub title: String,
    pub artist: String,
    pub art_url: Option<String>,
    pub status: PlaybackStatus,
    /// When the status or track last changed.
    pub last_change: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaCommand {
    Previous,
    PlayPause,
    Next,
}

/// The player to show: the most recently changed playing one, else the most
/// recently changed paused one. Stopped players are never shown.
pub fn pick_player(players: &[PlayerState]) -> Option<&PlayerState> {
    let latest = |status| {
        players
            .iter()
            .filter(|p| p.status == status)
            .max_by_key(|p| p.last_change)
    };
    latest(PlaybackStatus::Playing).or_else(|| latest(PlaybackStatus::Paused))
}

#[allow(dead_code)] // Used by the playing indicator on the panel button.
pub fn any_playing(players: &[PlayerState]) -> bool {
    players.iter().any(|p| p.status == PlaybackStatus::Playing)
}

/// Carry `last_change` over from `prev` for players whose status and track
/// did not change; the result contains exactly the players in `fresh`.
pub fn merge(prev: &[PlayerState], mut fresh: Vec<PlayerState>) -> Vec<PlayerState> {
    for p in &mut fresh {
        if let Some(old) = prev.iter().find(|o| o.bus_name == p.bus_name) {
            if old.status == p.status && old.title == p.title && old.artist == p.artist {
                p.last_change = old.last_change;
            }
        }
    }
    fresh
}

/// Local path for a `file://` art URL; `None` for remote or other schemes.
pub fn art_path(url: &str) -> Option<PathBuf> {
    let rest = url.strip_prefix("file://")?;
    let bytes = rest.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = bytes.get(i + 1..i + 3).and_then(|h| std::str::from_utf8(h).ok());
        if let (b'%', Some(Ok(b))) = (bytes[i], hex.map(|h| u8::from_str_radix(h, 16))) {
            out.push(b);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    use std::os::unix::ffi::OsStringExt;
    Some(PathBuf::from(std::ffi::OsString::from_vec(out)))
}

#[proxy(interface = "org.mpris.MediaPlayer2", default_path = "/org/mpris/MediaPlayer2")]
trait MediaPlayer2 {
    #[zbus(property)]
    fn identity(&self) -> zbus::Result<String>;
}

#[proxy(interface = "org.mpris.MediaPlayer2.Player", default_path = "/org/mpris/MediaPlayer2")]
trait Player {
    fn previous(&self) -> zbus::Result<()>;
    fn play_pause(&self) -> zbus::Result<()>;
    fn next(&self) -> zbus::Result<()>;

    #[zbus(property)]
    fn playback_status(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn metadata(&self) -> zbus::Result<HashMap<String, OwnedValue>>;
}

fn meta_str(meta: &HashMap<String, OwnedValue>, key: &str) -> Option<String> {
    meta.get(key).and_then(|v| <&str>::try_from(&**v).ok()).map(str::to_owned)
}

fn meta_artists(meta: &HashMap<String, OwnedValue>) -> String {
    match meta.get("xesam:artist").map(|v| &**v) {
        Some(Value::Array(a)) => a
            .iter()
            .filter_map(|x| <&str>::try_from(x).ok())
            .collect::<Vec<_>>()
            .join(", "),
        Some(v) => <&str>::try_from(v).map(str::to_owned).unwrap_or_default(),
        None => String::new(),
    }
}

const PLAYER_READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);

async fn read_player(conn: &Connection, bus_name: &str) -> zbus::Result<PlayerState> {
    let root = MediaPlayer2Proxy::builder(conn)
        .destination(bus_name.to_owned())?
        .cache_properties(CacheProperties::No)
        .build()
        .await?;
    let player = PlayerProxy::builder(conn)
        .destination(bus_name.to_owned())?
        .cache_properties(CacheProperties::No)
        .build()
        .await?;
    let identity = root.identity().await.unwrap_or_default();
    let status = PlaybackStatus::parse(&player.playback_status().await?);
    let meta = player.metadata().await.unwrap_or_default();
    Ok(PlayerState {
        bus_name: bus_name.to_owned(),
        identity,
        title: meta_str(&meta, "xesam:title").unwrap_or_default(),
        artist: meta_artists(&meta),
        art_url: meta_str(&meta, "mpris:artUrl").filter(|s| !s.is_empty()),
        status,
        last_change: Instant::now(),
    })
}

async fn read_players(conn: &Connection, dbus: &DBusProxy<'_>) -> zbus::Result<Vec<PlayerState>> {
    let mut out = Vec::new();
    for name in dbus.list_names().await? {
        let name = name.as_str();
        if !name.starts_with(MPRIS_PREFIX) || IGNORED.contains(&name) {
            continue;
        }
        // One hung player must not stall updates for the others.
        match tokio::time::timeout(PLAYER_READ_TIMEOUT, read_player(conn, name)).await {
            Ok(Ok(p)) => out.push(p),
            Ok(Err(err)) => tracing::debug!("Skipping MPRIS player {name}: {err}"),
            Err(_) => tracing::warn!("Skipping MPRIS player {name}: timed out reading it"),
        }
    }
    Ok(out)
}

async fn watch(output: &mut futures::channel::mpsc::Sender<Message>) -> zbus::Result<()> {
    let conn = Connection::session().await?;
    let dbus = DBusProxy::new(&conn).await?;
    let mut owners = dbus.receive_name_owner_changed().await?;
    let rule = MatchRule::builder()
        .msg_type(MessageType::Signal)
        .interface("org.freedesktop.DBus.Properties")?
        .member("PropertiesChanged")?
        .path(MPRIS_PATH)?
        .build();
    let mut props = MessageStream::for_match_rule(rule, &conn, Some(64)).await?;

    let mut players: Vec<PlayerState> = Vec::new();
    loop {
        players = merge(&players, read_players(&conn, &dbus).await?);
        if output.send(Message::Players(players.clone())).await.is_err() {
            return Ok(());
        }
        // Wait for something relevant to change.
        loop {
            tokio::select! {
                sig = owners.next() => {
                    let Some(sig) = sig else { return Ok(()) };
                    if sig.args().is_ok_and(|a| a.name().as_str().starts_with(MPRIS_PREFIX)) {
                        break;
                    }
                }
                msg = props.next() => {
                    if msg.is_none() { return Ok(()) }
                    break;
                }
            }
        }
        // Coalesce bursts (players often emit several changes at once).
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
}

/// Emits `Message::Players` with the full player list whenever a player
/// appears, disappears or changes its properties.
pub fn subscription() -> Subscription<Message> {
    struct Worker;
    Subscription::run_with(std::any::TypeId::of::<Worker>(), |_| {
        stream::channel(8, |mut output: futures::channel::mpsc::Sender<Message>| async move {
            loop {
                if let Err(err) = watch(&mut output).await {
                    tracing::error!("MPRIS watcher failed: {err}");
                }
                if output.is_closed() {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
        })
    })
}

/// Send a transport command to a player.
pub async fn send_command(bus_name: String, cmd: MediaCommand) {
    let result = async {
        let conn = Connection::session().await?;
        let player = PlayerProxy::builder(&conn)
            .destination(bus_name.clone())?
            .cache_properties(CacheProperties::No)
            .build()
            .await?;
        match cmd {
            MediaCommand::Previous => player.previous().await,
            MediaCommand::PlayPause => player.play_pause().await,
            MediaCommand::Next => player.next().await,
        }
    }
    .await;
    if let Err(err) = result {
        tracing::error!("MPRIS {cmd:?} on {bus_name} failed: {err}");
    }
}

/// Art (or a music icon), title, "artist · player" and transport buttons.
/// Renders nothing when there is no player to show.
pub fn view<'a>(players: &'a [PlayerState], art: Option<&'a icon::Handle>) -> Element<'a, Message> {
    let Some(p) = pick_player(players) else {
        return row![].into();
    };
    let art: Element<'a, Message> = match art {
        Some(h) => icon::icon(h.clone()).size(48).into(),
        None => container(icon::from_name("audio-x-generic-symbolic").size(32).symbolic(true).icon())
            .center(Length::Fixed(48.0))
            .into(),
    };
    let subtitle = match (p.artist.is_empty(), p.identity.is_empty()) {
        (false, false) => format!("{} · {}", p.artist, p.identity),
        (false, true) => p.artist.clone(),
        _ => p.identity.clone(),
    };
    let ctl = |name: &'static str, cmd| {
        button::icon(icon::from_name(name).size(16).symbolic(true))
            .on_press(Message::MediaCommand(p.bus_name.clone(), cmd))
    };
    let one_line = Ellipsize::End(EllipsizeHeightLimit::Lines(1));
    let play_icon = if p.status == PlaybackStatus::Playing {
        "media-playback-pause-symbolic"
    } else {
        "media-playback-start-symbolic"
    };
    row![
        art,
        column![
            text::body(p.title.as_str()).ellipsize(one_line).width(Length::Fill),
            text::caption(subtitle).ellipsize(one_line).width(Length::Fill),
        ]
            .spacing(2)
            .width(Length::Fill),
        ctl("media-skip-backward-symbolic", MediaCommand::Previous),
        ctl(play_icon, MediaCommand::PlayPause),
        ctl("media-skip-forward-symbolic", MediaCommand::Next),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn player(bus: &str, status: PlaybackStatus, age_secs: u64) -> PlayerState {
        PlayerState {
            bus_name: bus.into(),
            identity: "Test".into(),
            title: "Song".into(),
            artist: "Artist".into(),
            art_url: None,
            status,
            last_change: Instant::now() - Duration::from_secs(age_secs),
        }
    }

    #[test]
    fn prefers_playing_over_paused() {
        let paused_new = player("paused", PlaybackStatus::Paused, 1);
        let playing_old = player("playing", PlaybackStatus::Playing, 100);
        assert_eq!(pick_player(&[paused_new, playing_old]).unwrap().bus_name, "playing");
    }

    #[test]
    fn falls_back_to_most_recent_paused() {
        let older = player("older", PlaybackStatus::Paused, 100);
        let newer = player("newer", PlaybackStatus::Paused, 1);
        assert_eq!(pick_player(&[older.clone(), newer.clone()]).unwrap().bus_name, "newer");
        assert_eq!(pick_player(&[newer, older]).unwrap().bus_name, "newer");
    }

    #[test]
    fn stopped_players_hidden() {
        let stopped = player("stopped", PlaybackStatus::Stopped, 1);
        assert!(pick_player(&[stopped]).is_none());
    }

    #[test]
    fn removed_player_is_dropped() {
        let prev = vec![player("gone", PlaybackStatus::Playing, 1)];
        assert!(any_playing(&prev));
        let players = merge(&prev, Vec::new());
        assert!(pick_player(&players).is_none());
        assert!(!any_playing(&players));
    }

    /// Read-only check against the live session bus: `cargo test -- --ignored live_players`.
    #[tokio::test]
    #[ignore]
    async fn live_players() {
        let conn = Connection::session().await.unwrap();
        let dbus = DBusProxy::new(&conn).await.unwrap();
        let players = read_players(&conn, &dbus).await.unwrap();
        for p in &players {
            println!("{p:?}");
        }
        println!("picked: {:?}", pick_player(&players).map(|p| &p.bus_name));
    }

    #[test]
    fn art_path_only_for_file_urls() {
        assert_eq!(art_path("file:///tmp/a%20b.png"), Some(PathBuf::from("/tmp/a b.png")));
        assert_eq!(art_path("file:///tmp/100%"), Some(PathBuf::from("/tmp/100%")));
        assert_eq!(art_path("https://example.com/a.png"), None);
    }

    #[test]
    fn merge_keeps_last_change_when_unchanged() {
        let prev = vec![player("a", PlaybackStatus::Paused, 100)];
        let fresh = vec![player("a", PlaybackStatus::Paused, 0)];
        assert_eq!(merge(&prev, fresh)[0].last_change, prev[0].last_change);
    }

    #[test]
    fn merge_bumps_last_change_on_status_change() {
        let prev = vec![player("a", PlaybackStatus::Paused, 100)];
        let fresh = vec![player("a", PlaybackStatus::Playing, 0)];
        assert!(merge(&prev, fresh)[0].last_change > prev[0].last_change);
    }
}
