#!/usr/bin/env bash
#
# Синхронизирует OBS-проект home:arcticlore:apps/beamng-mod-downloader
# из репозитория packaging/. Выполнять после выпуска нового tag.
#
# Требования: пакет osc (openSUSE:Tools) и аккаунт на build.opensuse.org
#    osc -c ~/.config/osc/oscrc
#
set -euo pipefail

PROJECT="${OBS_PROJECT:-home:arcticlore:apps}"
PACKAGE="beamng-mod-downloader"
HERE="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
OBS_DIR="${OBS_DIR:-$HERE/.obs-checkout}"

log() { printf '\033[1;36m[obs]\033[0m %s\n' "$*"; }
die() { printf '\033[1;31m[obs] ERROR:\033[0m %s\n' "$*" >&2; exit 1; }

command -v osc >/dev/null 2>&1 || die "osc не установлен (openSUSE:Tools / python3-osc)"

log "обкатка: $OBS_DIR"
mkdir -p "$OBS_DIR"

# мета проекта (создаёт проект, если не существует)
osc meta prj "$PROJECT" -F "$HERE/packaging/obs/project.xml"

# обкатка пакета
if [ ! -d "$OBS_DIR/$PROJECT/$PACKAGE/.osc" ]; then
    ( cd "$OBS_DIR" && osc checkout "$PROJECT" "$PACKAGE" )
fi
DEST="$OBS_DIR/$PROJECT/$PACKAGE"

rm -rf "$DEST/debian"
cp -r "$HERE/packaging/debian" "$DEST/debian"
cp "$HERE/packaging/fedora/beamng-mod-downloader.spec" "$DEST/beamng-mod-downloader.spec"
cp "$HERE/packaging/obs/_service" "$DEST/_service"

cd "$DEST"
osc rm -f ./*.tar.* 2>/dev/null || true
if command -v tar_scm >/dev/null 2>&1 || osc service 2>/dev/null | grep -q .; then
    osc service run || log "предупреждение: локальный service run не прошёл (нет obs-service-tar_scm)"
fi
osc addremove
osc ci -m "sync: beamng-mod-downloader packaging" || true