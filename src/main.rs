// SPDX-License-Identifier: GPL-3.0-only

mod app;
mod calendar;
mod clock;
mod config;
mod localize;
mod notifications;

fn main() -> cosmic::iced::Result {
    // Default to info for this crate (fmt::init() alone filters everything below ERROR);
    // log to stderr, which cosmic-panel forwards to the journal.
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn,cosmic_ext_applet_clock_center=info"));
    tracing_subscriber::fmt().with_env_filter(filter).with_writer(std::io::stderr).init();
    localize::localize();
    tracing::info!("Starting clock-center applet {}", env!("CARGO_PKG_VERSION"));
    cosmic::applet::run::<app::App>(())
}
