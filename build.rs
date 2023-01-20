use vergen::{vergen, Config, TimeZone, TimestampKind};

fn main() {
    // set up the configuration for vergen
    let mut config = Config::default();
    *config.build_mut().kind_mut() = TimestampKind::DateAndTime;
    *config.build_mut().timezone_mut() = TimeZone::Utc;

    // configure vergen to generate the required environment variables
    if let Err(error) = vergen(config) {
        panic!(
            "Could not extract the required version information. The error was: {}",
            error
        );
    }
}
