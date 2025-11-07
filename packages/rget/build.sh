TERMUX_PKG_HOMEPAGE="https://github.com/Jobians/rust-resumable-downloader"
TERMUX_PKG_DESCRIPTION="A Rust downloader with resumable support and progress bar"
TERMUX_PKG_LICENSE="MIT"
TERMUX_PKG_MAINTAINER="@Jobians"
TERMUX_PKG_VERSION=0.1.0
TERMUX_PKG_REVISION=1
TERMUX_PKG_SRCURL=("https://github.com/Jobians/rust-resumable-downloader/archive/refs/tags/v0.1.0.tar.gz")
TERMUX_PKG_SHA256=("006a588a8b62572fec46eb42c355bc47115002411aeb6d9f89d997ff328ec1a7")
TERMUX_PKG_BUILD_IN_SRC=true
TERMUX_PKG_AUTO_UPDATE=false
TERMUX_PKG_DEPENDS="openssl"

termux_step_make() {
	termux_setup_rust
	cargo build --jobs $TERMUX_PKG_MAKE_PROCESSES --target $CARGO_TARGET_NAME --release
}

termux_step_make_install() {
	install -Dm700 -t $TERMUX_PREFIX/bin target/${CARGO_TARGET_NAME}/release/rget
}
