# cosmic-ext-applet-clock-center

One date/time button for the COSMIC panel: notifications, calendar, media and timers in one popup,
laid out like GNOME's date menu.

Screenshot coming soon.

## Features

- **Panel button**: the time/date label, using the settings of COSMIC's own time applet
  (`com.system76.CosmicAppletTime`: 24h, seconds, weekday, date, custom strftime format).
  An accent dot shows next to it while there are unread notifications, and four animated
  equalizer bars show while a media player is playing. Nothing extra is shown or animated otherwise.
- **Two-column popup**:
  - Left: media controls for the current MPRIS player (if any), then the notification list
    grouped per app as compact cards (expand long bodies, show older cards per app, dismiss one
    or a whole group, click to activate), with the **Do Not Disturb** toggle and **Clear** button
    pinned at the bottom.
  - Right: month calendar (respects your first day of week) and quick timers.
- **Quick timers**: one-click presets (default 1, 5, 10 and 25 minutes, configurable in
  `io.github.agustine_leo.CosmicExtAppletClockCenter`). A finished timer is sent as a
  **critical** notification. COSMIC shows no banners under Do Not Disturb, so while it is on the
  timer also plays the "complete" sound (`canberra-gtk-play`).
- Do Not Disturb is the same setting COSMIC uses (`com.system76.CosmicNotifications`).
- If the notifications socket is missing, the popup shows a warning instead of an empty list.

## Warning: replaces the stock notifications applet

This applet installs under the desktop id `com.system76.CosmicAppletNotifications`. Anywhere the panel is
configured to launch COSMIC's notifications applet, it launches this one instead, for your user only.
Remove it with `just uninstall` to get the stock applet back. Nothing is written outside `~/.local`.

## Install

Build dependencies on Fedora:

```sh
sudo dnf install cargo just pkgconf libxkbcommon-devel wayland-devel expat-devel fontconfig-devel \
    freetype-devel libinput-devel systemd-devel mesa-libEGL-devel
```

```sh
just install    # builds in release mode and installs to ~/.local
just uninstall  # removes the binary, desktop entry and icon
```

The new applet is picked up the next time the panel starts (log out and back in).

It shows its own clock, so remove COSMIC's time applet (`com.system76.CosmicAppletTime`) from the
panel centre, and Dynamic Island too if you use it, or you will see two clocks.

## Credits

Based on [pop-os/cosmic-applets](https://github.com/pop-os/cosmic-applets) (time and notifications applets),
GPL-3.0-only. Built with [libcosmic](https://github.com/pop-os/libcosmic).

## License

GPL-3.0-only
