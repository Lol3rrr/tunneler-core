macro_rules! debug {
    ($($arg:tt)+) => {
        #[cfg(feature = "logging")]
        log::debug!($($arg)+);
        #[cfg(feature = "trace")]
        tracing::debug!($($arg)+);
    };
}
macro_rules! info {
    ($($arg:tt)+) => {
        #[cfg(feature = "logging")]
        log::info!($($arg)+);
        #[cfg(feature = "trace")]
        tracing::info!($($arg)+);
    };
}
macro_rules! error {
    ($($arg:tt)+) => {
        #[cfg(feature = "logging")]
        log::error!($($arg)+);
        #[cfg(feature = "trace")]
        tracing::error!($($arg)+);
    };
}
