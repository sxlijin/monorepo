# Stems

Desktop multi-track stem player powered by a Rust audio engine and a Qt/QML front-end.

* Build: `cargo check`
* Run with auto-exit for capture/testing: `STEMS_DEV_TIMEOUT=10 cargo run --bin multi-player`
* Downloads and stem separation now happen entirely inside the app: the Rust bridge shells out to `uv run yt-dlp` for downloads and uses an embedded PyO3 separation module (see `src/analysis/stem_separation.py`).
