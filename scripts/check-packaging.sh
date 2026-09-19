#!/usr/bin/env bash
# Статическая валидация packaging-метаданных (PR gate, выполняется удалённо).
# Инварианты, которые обязаны соблюдаться всегда:
#  - desktop-файл валиден (desktop-file-validate, если установлен);
#  - лицензия MIT присутствует в LICENSE и spec;
#  - версия в spec/debian/changelog/dsc подставляется из template-переменной @PARENT_TAG@;
#  - template-файлы OBS на месте.
# Проверки, связанные с дистрибутивным релизом (нулевой sha256 в .dsc,
# sha256sums=('SKIP') в PKGBUILD), вводятся PR'ом packaging — см. docs.

set -euo pipefail
cd "$(dirname "$0")/.."

failures=0
fail() { echo "FAIL $1"; failures=$((failures + 1)); }
ok() { echo "OK   $1"; }

# --- desktop file ---
DESKTOP=packaging/beamng-mod-downloader.desktop
[ -f "$DESKTOP" ] || fail "нет $DESKTOP"
if grep -q '^\[Desktop Entry\]' "$DESKTOP" && grep -q '^Exec=' "$DESKTOP" &&
   grep -q '^Icon=' "$DESKTOP" && grep -q '^Categories=' "$DESKTOP"; then
  ok "desktop file: базовые секции на месте"
else
  fail "desktop file: отсутствует секция Exec/Icon/Categories/[Desktop Entry]"
fi
if command -v desktop-file-validate >/dev/null 2>&1; then
  desktop-file-validate "$DESKTOP" && ok "desktop-file-validate проходит"
fi

# --- license ---
[ -f LICENSE ] || fail "нет LICENSE"
grep -qi "MIT License" LICENSE || fail "LICENSE не MIT"
grep -q '^License: *MIT' packaging/fedora/beamng-mod-downloader.spec || fail "spec License != MIT"
ok "LICENSE MIT + spec License: MIT"

# --- version substitution через template-переменные ---
grep -q '^Version: *@PARENT_TAG@' packaging/fedora/beamng-mod-downloader.spec || fail "spec не использует @PARENT_TAG@"
grep -q 'beamng-mod-downloader (@PARENT_TAG@)' packaging/debian.changelog || fail "debian.changelog не использует @PARENT_TAG@"
grep -q '^Version: *@PARENT_TAG@' packaging/beamng-mod-downloader.dsc || fail "dsc не использует @PARENT_TAG@"
ok "template-переменные @PARENT_TAG@ в spec/debian.changelog/dsc"

# --- обязательные template/файлы сборки OBS ---
for f in packaging/obs/project.xml packaging/obs/config packaging/obs/_service packaging/obs-build.sh; do
  [ -f "$f" ] || fail "нет $f"
done
ok "OBS template-дерево на месте"

# --- MSRV в obs-build.sh согласован с Cargo.toml ---
APP_MSRV=$(grep -E '^rust-version' src-tauri/Cargo.toml | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1)
OBS_MSRV=$(grep -oE 'MIN_RUST="[0-9]+\.[0-9]+\.[0-9]+"' packaging/obs-build.sh | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1)
if [ -n "$APP_MSRV" ] && [ "$APP_MSRV" = "$OBS_MSRV" ]; then
  ok "MSRV согласован: $APP_MSRV"
else
  fail "MSRV рассинхронизирован: Cargo.toml='$APP_MSRV', obs-build.sh='$OBS_MSRV'"
fi

[ "$failures" -eq 0 ] || { echo "packaging static: $failures ошибок"; exit 1; }
echo "packaging static: OK"