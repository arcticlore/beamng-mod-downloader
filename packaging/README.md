# Пакетирование BeamNG Mod Downloader

Схема распространения: один источник правды — `packaging/`, из него
формируются пакеты для нескольких экосистем.

| Формат | Платформы | Канал |
| ------ | --------- | ----- |
| deb    | Ubuntu 22.04/24.04 + Debian 12/13 (+ производные: Mint, Pop!_OS…) | OBS `home:arcticlore:apps` |
| rpm    | Fedora 40/41 + openSUSE Tumbleweed/Leap 15.6 | OBS `home:arcticlore:apps` |
| pkg    | Arch (и производные: EndeavourOS…) | AUR `beamng-mod-downloader` (черновик) |
| AppImage/deb/rpm/nsis/msi/dmg | вся триада | GitHub Releases (`v0.1.1`) |

## Как работает сборка

- `obs-build.sh build` — единый сборщик: приводит тулчейн к рабочему состоянию
  (rust >= 1.77.2, node >= 18; если системные старше — rustup + nodejs.org LTS),
  делает `npm ci` + `vite build` + `cargo build --release` и раскладывает результат
  в `_pkg/` (bin + desktop + иконки).
- `obs-build.sh install <dir>` — копирует `_pkg/` в произвольный `dir` (для
  `debian/<pkg>`, `%{buildroot}`, `$pkgdir`).
- Способ сборки одинаков для всех, расхождение только в обёртках:
  `packaging/debian/`, `packaging/fedora/*.spec`, `packaging/arch/PKGBUILD`.

## OBS (deb + rpm)

Предпосылки:

1. Аккаунт на https://build.opensuse.org
2. `sudo zypper install -t pattern openSUSE_Tools` или
   `dnf install osc python3-osc` / `apt install osc`
3. `osc` базовая настройка: URL `https://api.opensuse.org`, user = логин, пароль.

Результат:

```sh
bash packaging/obs/push.sh          # создаёт проект, статус + пушит пакет
```

Скрипт применяет `packaging/obs/project.xml` (цели ниже) через `osc meta prj`,
кланяет пакет, раскладывает debian/ + spec + _service, запускает `osc service run`
(подтягивает `main` из GitHub через `tar_git`) и делает `osc ci`.

Цели по умолчанию (правятся прямо в `project.xml`):

- openSUSE Tumbleweed, openSUSE Leap 15.6
- Fedora 40, Fedora 41
- Ubuntu 22.04, Ubuntu 24.04
- Debian 12, Debian 13

> Если OBS не узнал `<path>` для Debian/Ubuntu — выбери правильную базу в веб-UI
> «Repositories → Add repository» (имена в `project.xml` — начальные значения).

Версия пакетов берётся из ближайшего git tag через `@PARENT_TAG@` (tar_git) —
выпустили `v0.1.1` → OBS соберёт `0.1.1`, следующая сборка после нового тега сама
возьмёт новую версию. Сборка мульти-дистро: один запушенный пакет даёт все repо.

### Подключение репозиториев у пользователей

**Ubuntu / Debian** (`xUbuntu_24.04`, `xUbuntu_22.04`, `Debian_12`, `Debian_13`):

```sh
curl -fsSL https://download.opensuse.org/repositories/home:/arcticlore:/apps/xUbuntu_24.04/Release.key \
  | sudo gpg --dearmor -o /usr/share/keyrings/beamng-mod-downloader.key
echo "deb [signed-by=/usr/share/keyrings/beamng-mod-downloader.key] https://download.opensuse.org/repositories/home:/arcticlore:/apps/xUbuntu_24.04/ /" \
  | sudo tee /etc/apt/sources.list.d/beamng-mod-downloader.list
sudo apt-get update && sudo apt-get install beamng-mod-downloader
```

**Fedora** (`Fedora_40`, `Fedora_41`) — OBS выдаёт готовый `.repo`:

```sh
sudo dnf config-manager addrepo --from-repofile=https://download.opensuse.org/repositories/home:/arcticlore:/apps/Fedora_41/home_arcticlore_apps.repo
sudo dnf install beamng-mod-downloader
```

**openSUSE** (Tumbleweed/Leap — `.repo` по той же схеме):

```sh
sudo zypper addrepo https://download.opensuse.org/repositories/home:/arcticlore:/apps/openSUSE_Tumbleweed/home_arcticlore_apps.repo
sudo zypper ref && sudo zypper install beamng-mod-downloader
```

## Arch

- `packaging/arch/PKGBUILD` — заготовка AUR-пакета (собирает из tag-релиза).
- Версия (`pkgver`) и хэш (`sha256sums`) обновляются перед релизом:
  `updpkgsums` после правки `pkgver`.
- Опционально: собрать отдельный pacman-репозиторий из GitHub Releases
  (`repo-add` в action) — тогда `pacman -S beamng-mod-downloader` без AUR.

## Замечания / TODO

- **Лицензия**: в репозитории нет `LICENSE`; пока во всех пакетах стоит `MIT`
  (debian/copyright помечен TODO). Определи лицензию до широкого релиза.
- **Ubuntu 22.04**: `libwebkit2gtk-4.1-dev` присутствует в jammy (universe) →
  цель `xUbuntu_22.04` работает; проект уже включает universe в path.
- **Тулчейн**: rust/node в buildroot ставятся с сети (rustup, nodejs.org).
  Для офлайн-сборок заложи пакеты дистрибутива (свежие Fedora/openSUSE/Arch
  проходят без скачивания).
- **AppImage** в CI строится только с `NO_STRIP=1` (linuxdeploy не понимает
  `.relr.dyn`); это уже учтено в `.github/workflows/build.yml`.
- Подпись пакетов (deb/apt/rpm) — OBS подписывает сама; публикация включена в
  `project.xml` (`<publish><enable/></publish>`).