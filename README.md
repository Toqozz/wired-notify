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
- **Layout** - position every element how you want it, see [wiki](https://github.com/Toqozz/wired-notify/wiki/Blocks) for more info.
- **Programmable and Interactable Layout Elements** - code your own or use layout elements from wired and contributors (accepting pull requests!).
    - Text blocks which scroll.
    - Backgrounds which reflect state (paused, active, extended, etc).
    - Layout elements can fire events on click (open url, etc).
    - More soon.
- **First Class Mouse Actions** - close, pause, and open urls within a notification with a click.
    - Open an issue if you have ideas of more actions.
- **Every notification is a different window** - pretty sick of stuff only being able to show one notification at a time honestly.

## Showcase
[Check out what other people have made with Wired!](https://github.com/Toqozz/wired-notify/issues/63)

## Config
See the [Config](https://github.com/Toqozz/wired-notify/wiki/Config) wiki page for configuration settings.

## Wiki
See [the wiki](https://github.com/Toqozz/wired-notify/wiki) for everything else you need to know about using Wired.

## Making your own elements
Making your own layout elements is designed to be as easy as possible.
Anybody who knows basic Rust should be able to make a layout element.
See [the wiki](https://github.com/Toqozz/wired-notify/wiki/Making-Your-Own-Blocks) for a detailed tutorial on making and adding a layout element to Wired.

## Building
### Dependencies
`rust, dbus, cairo, pango, glib2, x11, xss (for idle support)`
### Build and Run
```sh
$ git clone https://github.com/Toqozz/wired-notify.git
$ cd wired-notify
$ cargo build --release
$ ./target/release/wired
```

## Installing
### AUR
Wired is available on the [AUR](https://aur.archlinux.org/packages/wired/)!
```sh
$ yay -S wired
```

There's also a `-git` version which tracks master.  Beware!  No guarantees are made about stability on the master branch.  However, I do appreciate any help finding bugs before they make it to a release:
```sh
$ yay -S wired-git
```

### Nix (Flakes)
Flake support was added to Nix in version 2.4. As of Nix 2.8, you must enable the `nix-command` and `flakes` experimental features.

Wired can be run directly from the repository, using:
```sh
nix run 'github:Toqozz/wired-notify'
```

To install Wired to your user profile:
```sh
# note: the systemd service will not be available if installed with this method
nix profile install 'github:Toqozz/wired-notify'
```

To use it in another flake:
```nix
{
  inputs = {
    wired.url = github:Toqozz/wired-notify;
  };
}
```

For example, to install it for all users in NixOS:
```nix
{
  inputs = {
    nixpkgs.url = github:nixos/nixpkgs/nixpkgs-unstable;
    wired.url = github:Toqozz/wired-notify;
  };
  outputs = { self, nixpkgs, wired }: let
    std = nixpkgs.lib;
    system = "x86_64-linux";
  in {
    nixosConfigurations.alice = std.nixosSystem {
      inherit system;
      modules = [
        ./configuration.nix
        {
          environment.systemPackages = [ wired.packages.${system}.wired ];
        }
      ];
    };
  };
}
```

This flake also provides a module for [home-manager](https://github.com/nix-community/home-manager). To use it in your configuration:
```nix
{
  # ...
  outputs = { self, nixpkgs, home-manager, wired, ... }: {
    homeConfigurations.alice = let
      system = "x86_64-linux";
    in home-manager.lib.homeManagerConfiguration {
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ wired.overlays.${system} ];
      };

      modules = [
        wired.homeManagerModules.default
        ({ ... }: {
          services.wired = {
            enable = true;
            config = ./wired.ron;
          };
        })
      ];
    };
  };
}
```

### NetBSD
Wired is available from the official repositories,
```sh
$ pkgin install wired-notify
```
or, if you prefer to build from source
```sh
$ cd /usr/pkgsrc/x11/wired-notify
$ make install
```
### Fedora, CentOs and other RHEL-based distributions
Make sure you have DNF installed, and run the script with sudo permissions, otherwise the necessary dependencies cannot be installed.
```sh
$ cd wired-notify
$ chmod +x installer.sh
$ sudo ./installer.sh
```

## Running
The recommended way to start Wired is just to put the following in your autostart script:
```
/path/to/wired &
```

There is also a `wired.service` file in the root of the repository if you want to use systemd. Just copy it to `usr/local/lib/systemd/system` (or your distro equivalent) and run:
```
$ systemctl enable --now --user wired.service
```
