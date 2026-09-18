#!/usr/bin/env bash
#
# Единый сборщик дистрибутивных пакетов BeamNG Mod Downloader.
#
# Используется из debian/rules (deb-цели OBS), beamng-mod-downloader.spec
# (rpm-цели OBS / Fedora / openSUSE) и PKGBUILD (Arch). Скрипт сам приводит
# тулчейн к рабочему состоянию: если системные rust/node слишком старые,
# подтягивает свежие в $HOME (.cargo через rustup, node из nodejs.org).
#
# Способы запуска:
#   bash packaging/obs-build.sh build          # собрать бинарник и fill ../_pkg
#   bash packaging/obs-build.sh install <DIR>  # перенести _pkg под <DIR>/usr/*
#
# Требования на хосте/в чруте: bash, curl, xz (для node), gcc, pkg-config,
# webkit2gtk-4.1 + appindicator3 dev-пакеты дистрибутива.

set -euo pipefail

APP="beamng-mod-downloader"
MIN_RUST="1.77.2"        # MSRV Tauri 2
MIN_NODE_MAJOR="18"      # vite 6
NODE_LTS_POINT="latest-v20.x"

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
PKG_ROOT="_pkg"

log() { printf '\033[1;36m[obs-build]\033[0m %s\n' "$*"; }
die() { printf '\033[1;31m[obs-build] ERROR:\033[0m %s\n' "$*" >&2; exit 1; }

[ "$(uname -m)" = "x86_64" ] || die "поддерживается только x86_64 (текущая arch: $(uname -m))"

# --------------------------------------------------------------------------
# Версии тулчейна
# --------------------------------------------------------------------------
rust_ok() {
    command -v cargo >/dev/null 2>&1 || return 1
    local v major minor patch
    v="$(cargo --version 2>/dev/null | awk '{print $2}')" || return 1
    major="${v%%.*}"; tmp="${v#*.}"; minor="${tmp%%.*}"; patch="${tmp#*.}"
    [ "$major" -gt "${MIN_RUST%%.*}" ] && return 0
    [ "$major" -lt "${MIN_RUST%%.*}" ] && return 1
    local min_minor min_patch
    min_minor="$(echo "$MIN_RUST" | cut -d. -f2)"
    min_patch="$(echo "$MIN_RUST" | cut -d. -f3)"
    [ "$minor" -gt "$min_minor" ] && return 0
    [ "$minor" -lt "$min_minor" ] && return 1
    [ "$patch" -ge "$min_patch" ]
}

node_ok() {
    command -v node >/dev/null 2>&1 || return 1
    command -v npm >/dev/null 2>&1 || return 1
    local major
    major="$(node --version 2>/dev/null | tr -d 'v' | cut -d. -f1)"
    [ -n "$major" ] && [ "$major" -ge "$MIN_NODE_MAJOR" ]
}

setup_rust() {
    if rust_ok; then log "rust: нашли $(cargo --version | awk '{print $2}')"; return; fi
    log "rust: система старая или нет cargo, ставлю rustup (stable)"
    curl -fsSL https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable
    source "$HOME/.cargo/env"
    export CARGO_HOME="$HOME/.cargo"
    log "rust: $(cargo --version)"
    log "cargo: $(cargo --version)"
}

setup_node() {
    if node_ok; then log "node: нашли $(node --version) ($(npm --version))"; return; fi
    log "node: старше ${MIN_NODE_MAJOR} или нет npm, ставлю официальный node LTS"
    local base url tarball dir
    base="$(curl -fsSL "https://nodejs.org/dist/$NODE_LTS_POINT/SHASUMS256.txt" \
        | awk '/node-v[0-9.]+-linux-x64\.tar\.xz$/ {print $2; exit}')"
    [ -n "$base" ] || die "не смог определить номер версии node из SHASUMS256.txt"
    url="https://nodejs.org/dist/$NODE_LTS_POINT/$base"
    log "node: скачиваю $url"
    curl -fsSL "$url" | tar -xJ -C "$HOME"
    dir="${base%.tar.xz}"
    export PATH="$HOME/$dir/bin:$PATH"
    log "node: $(node --version)"
    log "npm: $(npm --version)"
}

# --------------------------------------------------------------------------
# Сборка и установка
# --------------------------------------------------------------------------
build_app() {
    cd "$ROOT"
    setup_rust
    setup_node

    # Офлайн-сборка: push.sh вшивает в tarball готовый dist/ и vendor/ (cargo).
    # В сетевом окружении (локально, без vendor/) собираем фронтенд сами.
    local OFFLINE=0
    [ -d "$ROOT/vendor" ] && [ -f "$ROOT/.cargo/config.toml" ] && OFFLINE=1
    [ -f "$ROOT/dist/index.html" ] && log "dist/ уже есть в tarball — npm ci/vite пропускаю" \
        || {
    log "npm ci"
    npm ci --no-audit --no-fund

    log "vite build (dist/)"
    npm run build
    }

    log "cargo build --release"
    export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"
    if [ "$OFFLINE" = 1 ]; then
        log "vendor/ найден — собираю офлайн"
        cargo build --release --offline --manifest-path "$ROOT/src-tauri/Cargo.toml"
    else
        cargo build --release --manifest-path "$ROOT/src-tauri/Cargo.toml"
    fi
    [ -x "$ROOT/src-tauri/target/release/$APP" ] || die "бинарник $APP не собран"

    log "staging -> $PKG_ROOT"
    rm -rf "$PKG_ROOT"
    mkdir -p "$PKG_ROOT/usr/bin" \
             "$PKG_ROOT/usr/share/applications" \
             "$PKG_ROOT/usr/share/licenses/$APP" \
             "$PKG_ROOT/usr/share/icons/hicolor/128x128/apps" \
             "$PKG_ROOT/usr/share/icons/hicolor/256x256/apps"

    install -m 0755 "$ROOT/src-tauri/target/release/$APP" "$PKG_ROOT/usr/bin/$APP"
    install -m 0644 "$ROOT/LICENSE" "$PKG_ROOT/usr/share/licenses/$APP/LICENSE"
    install -m 0644 "$ROOT/packaging/beamng-mod-downloader.desktop" \
        "$PKG_ROOT/usr/share/applications/beamng-mod-downloader.desktop"
    install -m 0644 "$ROOT/src-tauri/icons/128x128.png" \
        "$PKG_ROOT/usr/share/icons/hicolor/128x128/apps/beamng-mod-downloader.png"
    install -m 0644 "$ROOT/src-tauri/icons/128x128@2x.png" \
        "$PKG_ROOT/usr/share/icons/hicolor/256x256/apps/beamng-mod-downloader.png"

    log "готово. Артефакты в $PKG_ROOT/"
}

install_app() {
    local dest="${1:?нет каталога назначения}"
    cd "$ROOT"
    [ -d "$PKG_ROOT" ] || build_app
    log "install: копирую $PKG_ROOT/* -> $dest/"
    mkdir -p "$dest"
    cp -a "$PKG_ROOT/." "$dest/"
}

case "${1:-}" in
    build)    build_app ;;
    install)  install_app "${2:-}" ;;
    *)        die "использование: $0 {build|install <dir>}" ;;
esac