# enkei <a href="https://copr.fedorainfracloud.org/coprs/jwuensche/wayland-tools/package/enkei/"><img src="https://copr.fedorainfracloud.org/coprs/jwuensche/wayland-tools/package/enkei/status_image/last_build.png" /></a>
<a href="https://liberapay.com/spacesnek/donate"><img alt="Donate using Liberapay" src="https://liberapay.com/assets/widgets/donate.svg"></a>

`enkei` is a wallpaper tool created to allow displaying dynamic wallpapers based on the specification format used for example in the `Gnome` desktop environment.
It is designed to offer a _smooth_ transition between wallpapers and gradual change over long and short periods of time.
For a fast handling `enkei` uses `OpenGL` to render images and blending them for transitions.

Writing a wallpaper tool is nothing new so there are other projects which are similar.
There are already solutions for [animating GIFs](https://github.com/vilhalmer/oguri), [videos](https://github.com/GhostNaN/mpvpaper), [timed](https://github.com/xyproto/wallutils), and [static](https://github.com/swaywm/swaybg) wallpapers in Wayland.
But the use cases were quite different from what I wanted or I wasn't happy with their handling of animations, so I started writing `enkei`.

<video width="100%" controls muted loop alt="A video showing enkei running on my laptop.">
  <source src="data/demo.webm" type="video/webm">
</video>

![A video showing enkei running on my laptop.](data/demo.gif)

> gif created with [gifski](https://gif.ski)


## Features

- [X] Show static wallpapers
- [X] Show dynamic wallpapers
- [X] Scale images to fill and fit
- [X] Filter images after scaling to improve visuals
- [X] Change wallpaper on-the-fly with `enkeictl`
- [X] Support most common image formats (PNG, JPEG, WEBP, BMP,...)

> Under the hood we use [image](https://crates.io/crates/image) and [webp](https://github.com/jaredforth/webp) which provide the most common image types.

## Compositor Requirements

This tool can generally be used with all compositor implementing the core protocols and the following protocols:

- [wlr-layer-shell-unstable-v1](https://gitlab.freedesktop.org/wlroots/wlr-protocols/-/blob/master/unstable/wlr-layer-shell-unstable-v1.xml)

These are most of the time `wlroots` based compositors for example:

- [Sway](https://swaywm.org/)
- [Wayfire](https://wayfire.org/)
- [hikari](https://hikari.acmelabs.space/)
- [dwl](https://github.com/djpohly/dwl)

and many more; check an incomplete list here: https://github.com/solarkraft/awesome-wlroots#compositors

## Options

| flag | purpose/variants                                                               |
|:-----|:-------------------------------------------------------------------------------|
| `-f` | Filter Methods (Fast, Good, Best) [default: Good] |
| `-m` | Display Mode (Dynamic, Static) [default: Autodetect]                           |
| `-s` | Scale (Fill, Fit, None) [default: Fill]                                        |

`enkei` and `enkeictl` share these options. Specifying options in `enkeictl` will overwrite them in the running `enkei` instance.

## Installation

### Available Packages

#### Fedora, OpenSuse and CentOS

Prebuilt images are currently only available for Fedora 34/35/rawhide, OpenSuse Tumbleweed and Centos-stream 9 via [copr](https://copr.fedorainfracloud.org/coprs/jwuensche/wayland-tools/).

Add the copr to your system and install `enkei` with:

```shell
# dnf copr enable jwuensche/wayland-tools 
# dnf install enkei
```

#### ArchLinux

Self-build packages are available for archlinux
```shell
$ cd pkgs/archlinux
$ makepkg -i
```

#### Others

Patches for packaging of any other distribution are welcome, and will be merged gladly. 

### Manual

If no packages are available for your distribution, you want to manually install, or develop on `enkei` you can build it yourself locally with the following steps.

#### Requirements

You'll need the following dependencies to build this project.

```
git
cargo 
wayland-devel
mesa-libEGL-devel
glib2-devel
cairo-devel
cairo-gobject-devel
libwebp-devel
```

These are the package names on Fedora, for your favorite distribution they might differ.

### Building the project

To build the project then clone it and from within the cloned directory:

``` sh
$ cargo build
```

### Installing from Local Build

You can either install `enkei` via cargo, to your `$CARGO_BIN` directory

``` sh
$ cargo install --path .
```

or to any other arbitrary directory in your `$PATH` e.g. `/usr/local/bin`

``` sh
$ ./scripts/install.sh --prefix "/usr/local/bin"
```

> By default the `install.sh` script uses `/usr/bin`

### Using as a systemd-service

Under `config/enkei.service` an example service can be found to run sway as a systemd service.
How to run wayland specific daemons in systemd is well described in the [ArchWiki](https://wiki.archlinux.org/title/Sway#Manage_Sway-specific_daemons_with_systemd) though the article is sway specific this approach can be used in plenitude of scenarios.

## Related Projects

- [heic-to-dynamic-gnome-wallpaper](https://github.com/jwuensche/heic-to-dynamic-gnome-wallpaper)
- [dynamic-wallpaper-editor](https://github.com/maoschanz/dynamic-wallpaper-editor)
- [swaybg](https://github.com/swaywm/swaybg)
- [mpvpaper](https://github.com/GhostNaN/mpvpaper)
- [oguri](https://github.com/vilhalmer/oguri)

## Feature Ideas
    
- [ ] Individual wallpapers on different displays

    > The base functionality for this is already present, as each output is
    > treated individually. But logic and interface is missing to
    > realize this.
