# cosmic-ext-applet-clock-center

One date/time button for the COSMIC panel: notifications, calendar, media and timers in one popup.

Work in progress.

## Warning: replaces the stock notifications applet

This applet installs under the desktop id `com.system76.CosmicAppletNotifications`. Anywhere the panel is
configured to launch the stock notifications applet, it launches this one instead. Remove it with
`just uninstall` to get the stock applet back. Nothing is written outside `~/.local`.

## Install

```sh
just install    # builds, installs to ~/.local, then restart the panel: pkill -x cosmic-panel
just uninstall
```

## Credits

Based on [pop-os/cosmic-applets](https://github.com/pop-os/cosmic-applets) (time and notifications applets),
GPL-3.0-only. Built with [libcosmic](https://github.com/pop-os/libcosmic).

## License

GPL-3.0-only
