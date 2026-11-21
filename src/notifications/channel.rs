// Adapted from pop-os/cosmic-applets cosmic-applet-notifications/src/subscriptions/notifications.rs @ 328d142 (GPL-3.0-only)
// Adapted from pop-os/cosmic-applets cosmic-applet-notifications/src/subscriptions/dbus.rs @ 328d142 (GPL-3.0-only)
// Adapted from pop-os/cosmic-applets cosmic-applet-notifications/src/subscriptions/freedesktop_proxy.rs @ 328d142 (GPL-3.0-only)
// Copyright 2023 System76 <info@system76.com>
// SPDX-License-Identifier: GPL-3.0-only

use cosmic::iced::{futures, stream, Subscription};
use cosmic_notifications_util::Notification;
use futures_util::{FutureExt, SinkExt, StreamExt};
use std::{
    collections::HashMap,
    os::unix::io::{FromRawFd, RawFd},
    pin::pin,
};
use tokio::sync::mpsc;
use zbus::{connection::Builder, proxy};

use crate::app::Message;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChannelStatus {
    Connected,
    Unavailable(String),
}

/// Requests sent from the app to the notifications daemon connection.
#[derive(Debug, Clone)]
pub enum Input {
    /// Invoke an action (e.g. `"default"`) on the notification with this id.
    Activate(u32, String),
}

#[proxy(
    default_service = "com.system76.NotificationsApplet",
    interface = "com.system76.NotificationsApplet",
    default_path = "/com/system76/NotificationsApplet"
)]
pub trait NotificationsApplet {
    #[zbus(signal)]
    #[allow(clippy::too_many_arguments)]
    fn notify(
        &self,
        app_name: &str,
        id: u32,
        app_icon: &str,
        summary: &str,
        body: &str,
        actions: Vec<&str>,
        hints: HashMap<&str, zbus::zvariant::Value<'_>>,
        expire_timeout: i32,
    ) -> zbus::Result<()>;

    fn invoke_action(&self, id: u32, action: String) -> zbus::Result<()>;
}

#[proxy(
    interface = "org.freedesktop.Notifications",
    default_service = "org.freedesktop.Notifications",
    default_path = "/org/freedesktop/Notifications"
)]
pub trait Notifications {
    fn close_notification(&self, id: u32) -> zbus::Result<()>;

    #[zbus(signal)]
    fn notification_closed(&self, id: u32, reason: u32) -> zbus::Result<()>;
}

/// Connect to the notifications daemon over the socket cosmic-panel hands us
/// in `COSMIC_NOTIFICATIONS` (a raw fd number).
pub async fn get_proxy() -> Result<NotificationsAppletProxy<'static>, String> {
    let raw_fd = std::env::var("COSMIC_NOTIFICATIONS")
        .map_err(|e| format!("COSMIC_NOTIFICATIONS: {e}"))?;
    let raw_fd = raw_fd
        .parse::<RawFd>()
        .map_err(|e| format!("COSMIC_NOTIFICATIONS={raw_fd:?} is not an fd: {e}"))?;
    tracing::info!("Connecting to notifications daemon on fd {}", raw_fd);

    // SAFETY: cosmic-panel hands this socket fd to us; nothing else in the process uses it.
    let stream = unsafe { std::os::unix::net::UnixStream::from_raw_fd(raw_fd) };
    stream.set_nonblocking(true).map_err(|e| e.to_string())?;
    let stream = tokio::net::UnixStream::from_std(stream).map_err(|e| e.to_string())?;
    let conn = Builder::socket(stream)
        .p2p()
        .build()
        .await
        .map_err(|e| format!("p2p connection failed: {e}"))?;
    NotificationsAppletProxy::new(&conn)
        .await
        .map_err(|e| format!("proxy creation failed: {e}"))
}

async fn unavailable(output: &mut futures::channel::mpsc::Sender<Message>, reason: String) {
    tracing::error!("Notifications unavailable: {reason}");
    _ = output.send(Message::ChannelStatus(ChannelStatus::Unavailable(reason))).await;
}

/// Notifications from the daemon. Emits a sender for activation requests,
/// `ChannelStatus`, then one `Message::Notification` per notification.
pub fn subscription() -> Subscription<Message> {
    struct Worker;
    Subscription::run_with(std::any::TypeId::of::<Worker>(), |_| {
        stream::channel(50, |mut output: futures::channel::mpsc::Sender<Message>| async move {
            let proxy = match get_proxy().await {
                Ok(p) => p,
                Err(reason) => return unavailable(&mut output, reason).await,
            };
            let mut signal = match proxy.receive_notify().await {
                Ok(s) => s,
                Err(err) => {
                    return unavailable(&mut output, format!("cannot receive notifications: {err}")).await
                }
            };
            let (tx, mut rx) = mpsc::channel(10);
            _ = output.send(Message::NotificationsInput(tx)).await;
            _ = output.send(Message::ChannelStatus(ChannelStatus::Connected)).await;

            loop {
                let mut next_signal = signal.next();
                let mut next_input = pin!(rx.recv().fuse());
                futures::select! {
                    v = next_signal => {
                        let Some(msg) = v else {
                            unavailable(&mut output, "notifications daemon closed the connection".into()).await;
                            break;
                        };
                        let Ok(args) = msg.args() else { continue };
                        let n = Notification::new(
                            args.app_name,
                            args.id,
                            args.app_icon,
                            args.summary,
                            args.body,
                            args.actions,
                            args.hints,
                            args.expire_timeout,
                        );
                        tracing::info!(id = n.id, app = %n.app_name, summary = %n.summary, "Received notification");
                        _ = output.send(Message::Notification(n)).await;
                    }
                    v = next_input => {
                        let Some(Input::Activate(id, action)) = v else { break };
                        if let Err(err) = proxy.invoke_action(id, action.clone()).await {
                            tracing::error!("Failed to invoke action {action} for {id}: {err}");
                        }
                    }
                }
            }
        })
    })
}

/// Ids of notifications closed elsewhere (banner dismissed, app closed it).
pub fn closed_subscription() -> Subscription<Message> {
    struct Worker;
    Subscription::run_with(std::any::TypeId::of::<Worker>(), |_| {
        stream::channel(50, |mut output: futures::channel::mpsc::Sender<Message>| async move {
            let setup = async {
                let conn = zbus::Connection::session().await?;
                let proxy = NotificationsProxy::new(&conn).await?;
                proxy.receive_notification_closed().await
            };
            match setup.await {
                Ok(mut s) => {
                    while let Some(msg) = s.next().await {
                        if let Ok(args) = msg.args() {
                            _ = output.send(Message::NotificationClosed(args.id)).await;
                        }
                    }
                }
                Err(err) => tracing::error!("Cannot watch NotificationClosed: {err}"),
            }
        })
    })
}

/// Close notifications through org.freedesktop.Notifications on the session bus.
pub async fn close_notifications(ids: Vec<u32>) {
    if ids.is_empty() {
        return;
    }
    let setup = async {
        let conn = zbus::Connection::session().await?;
        NotificationsProxy::new(&conn).await
    };
    let proxy = match setup.await {
        Ok(p) => p,
        Err(err) => {
            tracing::error!("Cannot reach org.freedesktop.Notifications: {err}");
            return;
        }
    };
    for id in ids {
        if let Err(err) = proxy.close_notification(id).await {
            tracing::error!("Failed to close notification {id}: {err}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test] async fn missing_socket_reports_unavailable() {
        std::env::remove_var("COSMIC_NOTIFICATIONS");
        assert!(get_proxy().await.is_err());
    }
}
