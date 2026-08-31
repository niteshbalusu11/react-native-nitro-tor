#[macro_use]
pub extern crate log;

#[cfg(target_os = "android")]
use android_logger as logger;
#[cfg(not(target_os = "android"))]
use env_logger as logger;

pub struct Logger {}

impl Logger {
    pub fn new() -> Self {
        #[cfg(target_os = "android")]
        {
            logger::init_once(
                android_logger::Config::default()
                    .with_max_level(log::LevelFilter::Debug)
                    .with_tag("tor-ffi"),
            );
            info!("Android Logger init!");
        }
        #[cfg(not(target_os = "android"))]
        {
            let _ = logger::try_init();
        }

        log_panics::init(); // log panics rather than printing them
        info!("logging init");
        Logger {}
    }
}

impl Default for Logger {
    fn default() -> Self {
        Logger::new()
    }
}
