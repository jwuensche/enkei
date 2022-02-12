Name:           enkei
Version:        0.9.1
Release:        1%{?dist}
Summary:        A modern wallpaper tool with Gnome dynamic wallpaper support.

License:        GPL-3.0-only
URL:            https://enkei.spacesnek.rocks/
Source0:        https://codeload.github.com/jwuensche/%{name}/tar.gz/refs/tags/v%{version}

%if 0%{?suse_version}
BuildRequires:  cargo wayland-devel Mesa-libEGL-devel glib2-devel cairo-devel libwebp-devel gcc
Requires:       Mesa-libEGL1 libglib-2_0-0 libcairo2 libcairo-gobject2 libwebp7
%else
BuildRequires:  cargo wayland-devel mesa-libEGL-devel glib2-devel cairo-devel cairo-gobject-devel libwebp-devel
Requires:       mesa-libEGL glib2 cairo cairo-gobject libwebp
%endif

# I will not be bothered with this for now...
%global debug_package %{nil}

%description
%{summary}

%prep
%setup -q

%build
cargo build --release

%install
rm -rf $RPM_BUILD_ROOT
mkdir -p $RPM_BUILD_ROOT/%{_bindir}
cp target/release/%{name} $RPM_BUILD_ROOT/%{_bindir}
cp target/release/%{name}ctl $RPM_BUILD_ROOT/%{_bindir}

%files
%license COPYING
%{_bindir}/%{name}
%{_bindir}/%{name}ctl

%changelog
* Sun Feb 06 2022 v0.9.1 - Johannes <johannes@spacesnek.rocks>
  - Fix display behavior when using a scaling factors > 1
  - Use SIMD image resizing to cut down on startup time
* Sun Jan 30 2022 v0.9.0 - Johannes <johannes@spacesnek.rocks>
 - Release reimplementation v0.9.0
