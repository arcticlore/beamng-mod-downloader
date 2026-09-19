%global appname beamng-mod-downloader

# В vendor/ (cargo vendor в тигр) есть .rs-файлы, начинающиеся с `#![...]`,
# rpm считает их битыми shebang'ами и валит %install на brp-mangle-shebangs.
# Отключаем шаг целиком — в пакете нет скриптов со shebang (только ELF/PNG/desktop).
%global __brp_mangle_shebangs %{nil}

Name:           beamng-mod-downloader
Version:        @PARENT_TAG@
Release:        1%{?dist}
Summary:        Кроссплатформенный установщик модов для BeamNG.drive
License:        MIT
URL:            https://github.com/arcticlore/beamng-mod-downloader
Source0:        %{name}-%{version}.tar.xz

BuildRequires:  gcc-c++

# Тулчейн дистрибутива, где он достаточно свежий (Fedora, openSUSE).
# obs-build.sh использует системный cargo/node, если версия >= MSRV,
# иначе ставит свежие в $HOME (требует сети в билд-окружении).
%if 0%{?fedora} || 0%{?suse_version}
BuildRequires:  cargo
BuildRequires:  rust
BuildRequires:  nodejs
%endif
%if 0%{?fedora}
BuildRequires:  webkit2gtk4.1-devel
BuildRequires:  pkgconfig(ayatana-appindicator3-0.1)
BuildRequires:  npm
%else
BuildRequires:  typelib-1_0-JavaScriptCore-4_1
BuildRequires:  typelib-1_0-WebKit2-4_1
BuildRequires:  libayatana-appindicator-devel
BuildRequires:  pkgconfig(libsoup-3.0)
%endif
BuildRequires:  pkgconfig(gtk+-3.0)
BuildRequires:  librsvg2-devel
BuildRequires:  pkgconfig(openssl)
BuildRequires:  pkgconfig
BuildRequires:  curl

%description
Поиск папок модов BeamNG.drive, загрузка модов из нескольких источников
с прогрессом, просмотр имён, аватаров и описаний.

%prep
%setup -q -n %{name}-%{version}

%build
bash packaging/obs-build.sh build

%install
bash packaging/obs-build.sh install %{buildroot}

%files
%{_bindir}/%{appname}
%{_datadir}/applications/%{appname}.desktop
%{_datadir}/icons/hicolor/128x128/apps/%{appname}.png
%{_datadir}/icons/hicolor/256x256/apps/%{appname}.png
%{_datadir}/licenses/%{appname}/LICENSE

%changelog
* Thu Sep 18 2026 arcticlore <arcticlore@users.noreply.github.com> - @PARENT_TAG@-1
- Регулярная сборка из git-ветки main.