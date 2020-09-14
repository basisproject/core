//! A set of utilities for working with numbers in the Basis costs system.

/// Create a number.
///
/// This is mostly a wrapper around difference number types that makes it easier
/// to swap out test values/Costs types project-wide without having to change
/// each instance by hand, but can also be used by callers of the core to create
/// numbers more seamlessly.
#[macro_export]
macro_rules! num {
    ($val:expr) => {
        rust_decimal_macros::dec!($val)
    }
}

