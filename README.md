# enkei

A wayland wallpaper tool with support for Gnome dynamic wallpapers. The main motivation behind `enkei`
the option to display dynamic wallpapers compatible with the
Gnome wallpaper specification. This wallpaper tool uses [gtk-layer-shell](https://github.com/wmww/gtk-layer-shell) to render backgrounds
on all monitors. It has been tested in sway but should be compatible in general
with all compositors supporting the [wlr-layer-shell protocol](https://github.com/swaywm/wlr-protocols/blob/master/unstable/wlr-layer-shell-unstable-v1.xml).

<video width="100%" controls muted alt="A video showing a sped up desktop with enkei running.">
  <source src="data/demo.webm" type="video/webm">
</video>

## Features

- [X] Show Static Image
- [X] Show Dynamic Wallpapers
- [X] Set Different Scaling Modes
- [X] Set Wallpaper Filter Algorithm
- [X] Most common image formats supported (PNG, JPG, TIF,...)

> Under the hood we use [image](https://crates.io/crates/image) which provides the most common image types, have a look on their documentation

## Options

| flag | purpose/variants                                                               |
|:-----|:-------------------------------------------------------------------------------|
| `-f` | Filter Methods (Fast, Good, Best, Nearest, Bilinear, Gaussian) [default: Good] |
| `-m` | Display Mode (Dynamic, Static) [default: Autodetect]                           |
| `-s` | Scale (Fill, Fit, None) [default: Fill]                                        |

## Related Projects

- [swaybg](https://github.com/swaywm/swaybg)
- [mpvpaper](https://github.com/GhostNaN/mpvpaper)
- [oguri](https://github.com/vilhalmer/oguri)
- [heic-to-dynamic-gnome-wallpaper](https://github.com/jwuensche/heic-to-dynamic-gnome-wallpaper)

## Known Limitations / TODOs

- [ ] Handle Output Changes
    > It would be a nice to have to handle addition and removal of monitors
    > while running.  Changes required would be to connect to the gdk
    > DisplayManager and interact with these proclaimed changes. The delay in
    > which this should happen is best to be kept low, so long-time animations
    > will have to interrupted.

- [ ] More efficient image rendering for animation steps

    > We create new cairo surface for separate animation steps, this leads to
    > more effort in copying but guarantees a linear progression in the
    > animation.  We would best change this to only apply a certain level of
    > alpha to the image at each step halving the copy effort.
    
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
