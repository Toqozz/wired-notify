# Maintainer: Toqoz <https://github.com/Toqozz/wired-notify>

pkgname=wired-git
_pkgname=wired-notify
pkgver=r163.4460e60
pkgrel=1
pkgdesc="Lightweight notification daemon with highly customizable layout blocks, written in Rust."
arch=('x86_64' 'i686')
url="https://github.com/Toqozz/wired-notify"
license=('MIT')
depends=('dbus' 'cairo' 'pango' 'glib2' 'libx11' 'libxss')
makedepends=('rust' 'cargo')
provides=('wired')
conflicts=('wired')
source=("git://github.com/Toqozz/wired-notify/#branch=master")
sha256sums=('SKIP')

pkgver() {
    cd "${srcdir}/${_pkgname}"
    (
        set -o pipefail
        git describe --long 2>/dev/null | sed 's/\([^-]*-g\)/r\1/;s/-/./g' ||
            printf "r%s.%s" "$(git rev-list --count HEAD)" "$(git rev-parse --short HEAD)"
    )
}

build() {
    cd  "${srcdir}/${_pkgname}"
    cargo build --release --target-dir "./target"
}

package() {
    cd "${srcdir}/${_pkgname}"

    # Install binary.
    install -Dm 755 "target/release/wired" "${pkgdir}/usr/bin/wired"

    # Install MIT license
    install -Dm 644 LICENSE "${pkgdir}/usr/share/licenses/wired/LICENSE-MIT"
}
