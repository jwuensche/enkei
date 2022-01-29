# enkei
<a href="https://copr.fedorainfracloud.org/coprs/jwuensche/wayland-tools/package/enkei/"><img src="https://copr.fedorainfracloud.org/coprs/jwuensche/wayland-tools/package/enkei/status_image/last_build.png" /></a>

A wayland wallpaper tool with support for Gnome dynamic wallpapers.
The main motivation behind `enkei` is to display dynamic wallpapers compatible with the
Gnome wallpaper specification.

<video width="100%" controls muted loop alt="A video showing a sped up desktop with enkei running.">
  <source src="data/demo.webm" type="video/webm">
</video>

## Features

- [X] Show Static Image
- [X] Show Dynamic Wallpapers
- [X] Set Different Scaling Modes
- [X] Set Wallpaper Filter Algorithm
- [X] Most common image formats supported (PNG, JPG, TIF,...)

> Under the hood we use [image](https://crates.io/crates/image) which provides the most common image types, have a look on their documentation

## Compositor Requirements

This tool can generally be used with all compositor implementing the core protocols and the following protocols:

- [wlr-layer-shell](https://wayland.app/protocols/wlr-layer-shell-unstable-v1)

These are most of the time wlroots based compositors for example:

- [Sway](https://swaywm.org/)
- [Wayfire](https://wayfire.org/)
- [hikari](https://hikari.acmelabs.space/)
- [dwl](https://github.com/djpohly/dwl)

and many more check an uncomplete list here: https://github.com/solarkraft/awesome-wlroots#compositors

## Options

| flag | purpose/variants                                                               |
|:-----|:-------------------------------------------------------------------------------|
| `-f` | Filter Methods (Fast, Good, Best, Nearest, Bilinear, Gaussian) [default: Good] |
| `-m` | Display Mode (Dynamic, Static) [default: Autodetect]                           |
| `-s` | Scale (Fill, Fit, None) [default: Fill]                                        |

## Installation

### Available Packages

Prebuilt images are currently only available for Fedora 34/35/rawhide via [copr](https://copr.fedorainfracloud.org/coprs/jwuensche/wayland-tools/).

Add the copr to your system and install `enkei` with:

```shell
# dnf copr enable jwuensche/wayland-tools 
# dnf install enkei
```

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

You can either install `enkei` via a cargo, to your `$CARGO_BIN` directory

``` sh
$ cargo install --path .
```

or to any other arbitrary directory in your path e.g. `/usr/local/bin`.

``` sh
$ cargo build --release
$ install -Dm755 "target/release/enkei" "/usr/local/bin/enkei"
```


## Related Projects

- [swaybg](https://github.com/swaywm/swaybg)
- [mpvpaper](https://github.com/GhostNaN/mpvpaper)
- [oguri](https://github.com/vilhalmer/oguri)
- [heic-to-dynamic-gnome-wallpaper](https://github.com/jwuensche/heic-to-dynamic-gnome-wallpaper)

## Feature Ideas
  
- [ ] Allow setting of wallpapers via IPC

    > A nice to have would be to send messages to the already running enkei
    > instance to change the wallpaper shown.  For this we would need to
    > interrupt any ongoing animations or static images and hot-swap the images
    > in the current gtk session.

    > oguri has done something similar seems like a cool feature to have to
    > avoid respawning the application.
    
- [ ] Individual wallpapers on different displays

    > The base functionality for this is already present, as each output is
    > treated individually. But higher logic and interface is missing to
    > realize this.  This goes hand in hand with being able to choose on which
    > display you want to display a wallpaper. Maybe not all should be set.
