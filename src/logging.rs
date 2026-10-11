use log::LevelFilter;
use std::sync::OnceLock;

static LOGGER_INIT: OnceLock<()> = OnceLock::new();

pub fn init_logger() {
    LOGGER_INIT.get_or_init(|| {
        android_logger::init_once(
            android_logger::Config::default()
                .with_max_level(LevelFilter::Trace)
                .with_tag("OhMyKeymint"),
        );
        log::set_max_level(LevelFilter::Debug);
    });
}
