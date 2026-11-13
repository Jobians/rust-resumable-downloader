TERMUX_PKG_HOMEPAGE="https://github.com/Jobians/rust-resumable-downloader"
TERMUX_PKG_DESCRIPTION="A Rust downloader with resumable support and progress bar"
TERMUX_PKG_LICENSE="MIT"
TERMUX_PKG_MAINTAINER="@Jobians"
TERMUX_PKG_VERSION=0.1.1
TERMUX_PKG_REVISION=1
TERMUX_PKG_SRCURL=("https://github.com/Jobians/rust-resumable-downloader/archive/refs/tags/v0.1.1.tar.gz")
TERMUX_PKG_SHA256=("9b9b141f7850f880d730c5821dd265eead4cea43508fc3fdeb18771d3cc52a74")
TERMUX_PKG_BUILD_IN_SRC=true
TERMUX_PKG_AUTO_UPDATE=false

termux_step_make() {
	termux_setup_rust
	cargo build --jobs $TERMUX_PKG_MAKE_PROCESSES --target $CARGO_TARGET_NAME --release
}

termux_step_make_install() {
	install -Dm700 -t $TERMUX_PREFIX/bin target/${CARGO_TARGET_NAME}/release/rget
}
