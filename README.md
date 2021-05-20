# enkei (遠景)

A modern wallpaper tool with support for Gnome dynamic wallpapers. The main motivation behind `enkei` was
the possibility to display dynamic and changing wallpapers compatible with the
Gnome wallpaper specification. This wallpaper tool uses
[gtk-layer-shell](https://github.com/wmww/gtk-layer-shell) to render backgrounds
on all monitors. It has been tested in sway but should be compatible in general
with all compositors supporting the [wlr-layer-shell protocol](https://github.com/swaywm/wlr-protocols/blob/master/unstable/wlr-layer-shell-unstable-v1.xml).

## Features

- [X] Show Static Image
- [X] Show Dynamic Wallpapers
- [ ] Set Different Scaling Modes
- [ ] Set Wallpaper Filter Algorithm

## Known Limitations / TODOs

- [ ] Support more image types

    > We use cairo at the moment to parse image files, due to this we only
    > support pngs as these have official integration within cairo.  Extension
    > could be made by adding additional parsers which then simply forward the
    > color channels and dimensions to cairo.  Pull requests are welcome.
    
- [ ] More efficient image rendering for animation steps

    > We create new cairo surface for separate animation steps, this leads to
    > more effort in copying but guarantees a linear progression in the
    > animation.  We would best change this to only apply a certain level of
    > alpha to the image at each step halving the copy effort. But this leads
    > with simple scaling to a skew in the animation rate. Looking for good
    > ideas for this right now.
    
- [ ] Allow setting of wallpapers via IPC

    > A nice to have would be to send messages to the already running enkei
    > instance to change the wallpaper shown.  For this we would need to
    > interrupt any ongoing animations or static images and hot-swap the images
    > in the current gtk session.
    
- [ ] Individual wallpapers on different displays

    > The base functionality for this is already present, as each output is
    > treated individually. But higher logic and interface is missing to realize
    > this.
