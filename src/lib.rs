//! Useful macros for working with timeouts on top of `embedded-hal` APIs
//!
//! The non-blocking APIs in the [`embedded-hal`] crate use `nb::Result` from
//! the [`nb`] crate to signal whether an operation has finished. This can be
//! a bit tedious to work with in non-trivial cases, for example if timeouts are
//! involved.
//!
//! This crate defines macros that help working with timeouts on top of
//! `embedded-hal` APIs.
//!
//! [`embedded-hal`]: https://crates.io/crates/embedded-hal
//! [`nb`]: https://crates.io/crates/nb


#![no_std]

#![deny(missing_docs)]


pub use embedded_hal;
pub use nb;


/// Blocks on a non-blocking operation until a timer times out
///
/// Expects two arguments:
///
/// - A timer that implements `embedded_hal::timer::CountDown`
/// - An expression that evaluates to `nb::Result<T, E>`
///
/// Evaluates the expression and returns `Result<T, TimeoutError<E>>`.
///
/// # Example
///
/// ``` rust
/// use embedded_timeout_macros::{
///     block_timeout,
///     TimeoutError,
/// };
/// #
/// # struct Timer;
/// #
/// # impl embedded_hal::timer::CountDown for Timer {
/// #     type Time = ();
/// #     fn start<T>(&mut self, _: T) {}
/// #     fn wait(&mut self) -> nb::Result<(), void::Void> { Ok(()) }
/// # }
/// #
/// # let mut timer = Timer;
///
/// let result: Result<(), TimeoutError<()>> = block_timeout!(
///     &mut timer,
///     {
///         // The macro will keep evaluation this expression repeatedly until
///         // it returns `Ok` or until the timer times out.
///         //
///         // We can do anything that returns `nb::Result` here. For this
///         // simple example, we just return `Ok`.
///         Ok(())
///     }
/// );
///
/// match result {
///     Ok(()) => {
///         // success
///     }
///     Err(TimeoutError::Timeout) => {
///         // the operation timed out
///     }
///     Err(TimeoutError::Other(error)) => {
///         // the operation returned another error
///     }
/// }
/// ```
#[macro_export]
macro_rules! block_timeout {
    ($timer:expr, $op:expr) => {
        {
            use $crate::embedded_hal::prelude::*;

            // Make sure the timer has the right type. If it isn't, the user
            // should at least get a good error message.
            fn check_type<T>(_: &mut T)
                where T: $crate::embedded_hal::timer::CountDown {}
            check_type($timer);

            loop {
                match $timer.wait() {
                    Ok(()) =>
                        break Err($crate::TimeoutError::Timeout),
                    Err($crate::nb::Error::WouldBlock) =>
                        (),
                    Err(_) =>
                        unreachable!(),
                }

                match $op {
                    Ok(result) =>
                        break Ok(result),
                    Err($crate::nb::Error::WouldBlock) =>
                        (),
                    Err($crate::nb::Error::Other(error)) =>
                        break Err($crate::TimeoutError::Other(error)),
                }
            }
        }
    }
}

/// Repeats an operation until a timer times out
///
/// Expects four arguments:
///
/// - A timer that implements `embedded_hal::timer::CountDown`
/// - An expression that evaluates to `Result<T, E>` (the operation)
/// - A closure that will be called every time the operation succeeds
///   This closure is expected to take an argument of type `T`. The return value
///   is ignored.
/// - A closure that will be called every time the operation fails
///   This closure is expected to take an argument of type `E`. The return value
///   is ignored.
///
/// This will keep repeating the operation until the timer runs out, no matter
/// whether it suceeds or fails.
///
/// # Example
///
/// ``` rust
/// use embedded_timeout_macros::{
///     repeat_timeout,
///     TimeoutError,
/// };
/// #
/// # struct Timer;
/// #
/// # impl embedded_hal::timer::CountDown for Timer {
/// #     type Time = ();
/// #     fn start<T>(&mut self, _: T) {}
/// #     fn wait(&mut self) -> nb::Result<(), void::Void> { Ok(()) }
/// # }
/// #
/// # let mut timer = Timer;
///
/// repeat_timeout!(
///     &mut timer,
///     {
///         // The macro will keep evaluation this expression repeatedly until
///         // the timer times out.
///         //
///         // We can do anything that returns `Result` here. For this simple
///         // example, we just return `Ok`.
///         Ok(())
///
///         // We could also return an error.
///         // Err("This is an error")
///     },
///     |result: ()| {
///         // will be called by the macro, if the expression returns `Ok`
///     },
///     |error: &str| {
///         // will be called by the macro, if the expression returns `Err`
///     },
/// );
/// ```
#[macro_export]
macro_rules! repeat_timeout {
    ($timer:expr, $op:expr, $on_success:expr, $on_error:expr,) => {
        {
            use $crate::embedded_hal::prelude::*;

            // Make sure the timer has the right type. If it isn't, the user
            // should at least get a good error message.
            fn check_type<T>(_: &mut T)
                where T: $crate::embedded_hal::timer::CountDown {}
            check_type($timer);

            loop {
                match $timer.wait() {
                    Ok(()) =>
                        break,
                    Err($crate::nb::Error::WouldBlock) =>
                        (),
                    Err(_) =>
                        unreachable!(),
                }

                match $op {
                    Ok(result) => {
                        $on_success(result);
                    }
                    Err(error) => {
                        $on_error(error);
                    }
                }
            }
        }
    }
}


/// An error that can either be a timeout or another error
///
/// Returned by the [`block_timeout`] macro.
#[derive(Debug)]
pub enum TimeoutError<T> {
    /// The operation timed out
    Timeout,

    /// Another error occured
    Other(T),
}
