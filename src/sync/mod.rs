#[cfg(feature = "std")]
mod mutex;
#[cfg(feature = "std")]
mod poison;

#[cfg(feature = "std")]
pub use mutex::*;
