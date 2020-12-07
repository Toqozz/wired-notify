<p align="middle">
  <img src="https://raw.githubusercontent.com/Toqozz/wired-notify/master/readme_stuff/musicc.gif" width="400" align="left">
  <div>
    <img src="https://raw.githubusercontent.com/Toqozz/wired-notify/master/readme_stuff/simple.gif" width="400" align="top"/>
    <br>
    <img src="https://raw.githubusercontent.com/Toqozz/wired-notify/master/readme_stuff/horizontal.gif" width="300" align="top"/>
  </div>
</p>
<br><br><br><br><br><br>

---

# Wired
Wired is light and fully customizable notification daemon that provides you with powerful and extensible layout
tools.

## Features
- **Layout** - position everything how you want it, through a pretty clunky configuration file right now.
- **Programmable Layout Elements** - code your own or use layout elements from wired and contributors (accepting pull requests!).
    - Text blocks which scroll.
    - Backgrounds which reflect state (paused, active, extended, etc).
    - More soon.
- **First Class Mouse Actions** - close, pause, and open urls within a notification with a click.
    - Open an issue if you have ideas of more actions.
- **Every notification is a different window** - pretty sick of stuff only being able to show one notification at a time honestly.

## Making your own elements
Making your own layout elements is designed to be as easy as possible.
Anybody who knows basic Rust should be able to make a layout element.
See [the wiki](https://github.com/Toqozz/wired-notify/wiki/Making-Your-Own-Blocks) for a detailed tutorial on making and adding a layout element to Wired.

## Building
### Dependencies
`rust, dbus, cairo, pango`
### Build and Run
```
$ git clone https://github.com/Toqozz/wired-notify.git
$ cd wired-notify
$ cargo build --release
$ ./target/release/wired
```

## Wiki
See [the wiki](https://github.com/Toqozz/wired-notify/wiki) for everything else you need to know about using Wired.

## Wired is not finished, but it is usable for most people.
There's a bunch of things that aren't done yet; here's a non-exhaustive version of the TODO list:
- [x] Make config not as painful.
- [x] Allow hex colors in config.
- [x] `%t` for time, etc, in text blocks.
- [ ] Lookup application icons via `.desktop` file
- [ ] Support `replaces_id` functionality.
- [ ] More options surrounding notification urgency.
- [ ] Notification follows active monitor.
- [ ] Tests...
- [ ] Random html escape code edge cases.
