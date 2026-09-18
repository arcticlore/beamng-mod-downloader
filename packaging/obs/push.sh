#!/usr/bin/env bash
#
# Синхронизирует OBS-проект home:arcticlore:apps/beamng-mod-downloader
# из этого репозитория. Выполнять после выпуска нового git-тега:
#   bash packaging/obs/push.sh
#
# Схема: никакой _service на сервере — tarball собирается локально
# (git archive из последнего тега) и загружается вместе с пакетными
# файлами. @PARENT_TAG@ в шаблонах заменяется на фактический тег.
# Работает через рабочую копию osc (osc checkout + ci --noservice).
#
# Требования: osc (openSUSE:Tools) + креды в ~/.config/osc/oscrc.
#
set -euo pipefail

PROJECT="${OBS_PROJECT:-home:arcticlore:apps}"
PACKAGE="beamng-mod-downloader"
HERE="$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)"
VERSION="$(git -C "$HERE" describe --tags --abbrev=0 2>/dev/null | sed 's/^v//')"
API="${OBS_APIURL:-https://api.opensuse.org}"
WC="$(mktemp -d)"
STAGE="$(mktemp -d)"
trap 'rm -rf "$WC" "$STAGE"' EXIT

log() { printf '\033[1;36m[obs]\033[0m %s\n' "$*"; }
die() { printf '\033[1;31m[obs] ERROR:\033[0m %s\n' "$*" >&2; exit 1; }

[ -n "$VERSION" ] || die "нет git-тегов — неоткуда взять версию"
command -v osc >/dev/null 2>&1 || die "osc не установлен (openSUSE:Tools / python3-osc)"
log "версия: $VERSION"

# --- проект/пакет ----------------------------------------------------------
osc -A "$API" meta prj "$PROJECT" -F "$HERE/packaging/obs/project.xml"
if [ -f "$HERE/packaging/obs/config" ]; then
    osc -A "$API" meta prjconf "$PROJECT" -F "$HERE/packaging/obs/config"
fi
if ! osc -A "$API" api "/source/$PROJECT/$PACKAGE" >/dev/null 2>&1; then
    log "создаю пакет $PACKAGE"
    P="$STAGE/pkg.meta.xml"
    cat > "$P" <<EOF
<package name="$PACKAGE" project="$PROJECT">
  <title>BeamNG Mod Downloader</title>
  <description>Кроссплатформенный установщик модов для BeamNG.drive</description>
</package>
EOF
    osc -A "$API" api -X PUT -f "$P" "/source/$PROJECT/$PACKAGE/_meta" >/dev/null
fi

# --- tarball из git + dist/ + vendor/ (офлайн-сборка на OBS) ---------------
TAR="$PACKAGE-$VERSION.tar.xz"
log "собираю $TAR (git archive + dist/ + vendor/)"
SRC="$STAGE/$PACKAGE-$VERSION"
mkdir -p "$SRC"
git -C "$HERE" archive HEAD | tar -x -C "$SRC"

log "  vite (dist/)"
(cd "$SRC" && npm ci --no-audit --no-fund && npm run build)

log "  cargo vendor"
(cd "$SRC" && cargo vendor --locked vendor >/dev/null)

cat > "$SRC/.cargo/config.toml" <<'EOF'
[source.crates-io]
replace-with = "vendored-sources"

[source.vendored-sources]
directory = "vendor"
EOF

tar -cJf "$STAGE/$TAR" -C "$STAGE" \
    --exclude="$PACKAGE-$VERSION/node_modules" \
    "$PACKAGE-$VERSION"
[ -s "$STAGE/$TAR" ] || die "git archive не дал результата"

# --- шаблоны с версией ------------------------------------------------------
sed "s/@PARENT_TAG@/$VERSION/g" "$HERE/packaging/fedora/beamng-mod-downloader.spec" > "$STAGE/beamng-mod-downloader.spec"
sed "s/@PARENT_TAG@/$VERSION/g" "$HERE/packaging/beamng-mod-downloader.dsc" > "$STAGE/beamng-mod-downloader.dsc"
for f in debian.changelog debian.compat debian.control debian.rules; do
    sed "s/@PARENT_TAG@/$VERSION/g" "$HERE/packaging/$f" > "$STAGE/$f"
done

# --- рабочая копия + загрузка -------------------------------------------------
log "обкатка и upload в $PROJECT/$PACKAGE"
cd "$WC"
osc -A "$API" checkout "$PROJECT" "$PACKAGE" >/dev/null
WG="$WC/$PROJECT/$PACKAGE"

log "  удаляю _service (не используем) и мусор"
rm -f "$WG/_service" "$WG/t2.txt" "$WG/probe.txt"

log "  копирую файлы"
for f in beamng-mod-downloader.spec beamng-mod-downloader.dsc \
         debian.changelog debian.compat debian.control debian.rules "$TAR"; do
    cp "$STAGE/$f" "$WG/$f"
done

cd "$WG"
osc rm -f _service t2.txt probe.txt 2>/dev/null || true
osc addremove
osc ci --noservice -m "beamng-mod-downloader $VERSION" 2>&1 \
    | tail -12

log "готово: https://build.opensuse.org/package/show/$PROJECT/$PACKAGE"
osc -A "$API" results "$PROJECT" "$PACKAGE" 2>/dev/null || true