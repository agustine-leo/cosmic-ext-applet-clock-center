name := 'cosmic-ext-applet-clock-center'
appid := 'io.github.agustine_leo.CosmicExtAppletClockCenter'
desktop-id := 'com.system76.CosmicAppletNotifications'

prefix := env('HOME') / '.local'
bin-dst := prefix / 'bin' / name
desktop-dst := prefix / 'share' / 'applications' / desktop-id + '.desktop'
icon-dst := prefix / 'share' / 'icons' / 'hicolor' / 'scalable' / 'apps' / appid + '.svg'

default: build-release

build-release *args:
    cargo build --release --locked {{args}}

install: build-release
    install -Dm0755 target/release/{{name}} {{bin-dst}}
    install -Dm0644 data/{{desktop-id}}.desktop {{desktop-dst}}
    install -Dm0644 data/icons/scalable/apps/{{appid}}.svg {{icon-dst}}
    sed -i 's|^Exec=.*|Exec={{bin-dst}}|' {{desktop-dst}}

uninstall:
    rm -f {{bin-dst}} {{desktop-dst}} {{icon-dst}}
