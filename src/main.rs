// SPDX-License-Identifier: GPL-3.0-only

mod app;
mod calendar;
mod clock;
mod config;
mod localize;

fn main() -> cosmic::iced::Result {
    tracing_subscriber::fmt::init();
    localize::localize();
    tracing::info!("Starting clock-center applet {}", env!("CARGO_PKG_VERSION"));
    cosmic::applet::run::<app::App>(())
}
