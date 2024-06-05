# Maintainer: Toqoz <https://github.com/Toqozz/wired-notify>

pkgname=wired
_pkgname=wired-notify
pkgver=0.10.6
pkgrel=1
pkgdesc="Lightweight notification daemon with highly customizable layout blocks, written in Rust."
arch=('x86_64' 'i686')
url="https://github.com/Toqozz/wired-notify"
license=('MIT')
depends=('dbus' 'cairo' 'pango' 'glib2' 'libx11' 'libxss')
makedepends=('rust' 'cargo')
provides=('wired')
conflicts=('wired')
source=("${pkgname}-${pkgver}.tar.gz::https://github.com/Toqozz/wired-notify/archive/${pkgver}.tar.gz")
sha256sums=('4a642afb7edf25e2735ec41e72dfb538769233832ec2332a2a12d993cc04f99c')


build() {
    cd  "${srcdir}/${_pkgname}-${pkgver}"
    cargo build --release --target-dir "./target"
}

package() {
    cd "${srcdir}/${_pkgname}-${pkgver}"

    # Install binary.
    install -Dm 755 "target/release/${pkgname}" "${pkgdir}/usr/bin/${pkgname}"

    # Install MIT license
    install -Dm 644 LICENSE "${pkgdir}/usr/share/licenses/${pkgname}/LICENSE-MIT"
}
