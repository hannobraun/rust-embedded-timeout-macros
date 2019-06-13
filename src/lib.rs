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
//! # Why Macros?
//!
//! A similar result could be achieved using functions and closures, so "Why use
//! macros for this?" is a fair question. While macros can be confusing and hard
//! to work with, they are also more powerful. Here are some things you can do
//! using the macros in this crate, that you couldn't do with a
//! function/closure-based approach:
//!
//! - You can `return` the current function, from within the closure.
//! - You can `break`/`continue` an outer loop, from within the closure.
//! - You can rely on the compiler's reasoning to support complicated moves.
//!
//! That last point is the least obvious, so it deserves some elaboration. Take
//! the following example:
//!
//! ``` no_run
//! let mut thing_being_idle = Thing::new();
//!
//! loop {
//!     // `do_stuff` takes ownership of the idle thing and returns a new type
//!     // that represents the same thing, but no longer being idle and doing
//!     // stuff.
//!     let thing_doing_stuff = thing_being_idle.do_stuff();
//!
//!     // stuff is being done
//!
//!     // `finish_doing_stuff` again takes ownership of the thing, and returns
//!     // the original type that represents the thing while it's being idle.
//!     // We move that new idle thing into the original variable.
//!     //
//!     // The compiler understands this, so even though we've moved out of
//!     // `thing_being_idle`, it's going to be available again in the next loop
//!     // iteration.
//!     thing_being_idle = thing_doing_stuff.finish_doing_stuff();
//! }
//!
//!
//! struct Thing<State>(State);
//!
//! impl Thing<BeingIdle> {
//!     fn new() -> Self {
//!         Thing(BeingIdle)
//!     }
//!
//!     fn do_stuff(self) -> Thing<DoingStuff> {
//!         // Start doing the important stuff
//!         // ...
//!
//!         Thing(DoingStuff)
//!     }
//! }
//!
//! impl Thing<DoingStuff> {
//!     fn finish_doing_stuff(self) -> Thing<BeingIdle> {
//!         // Finish doing the important stuff
//!         // ...
//!
//!         Thing(BeingIdle)
//!     }
//! }
//!
//! struct BeingIdle;
//! struct DoingStuff;
//! ```
//!
//! Since the macros in this crate are basically just fancy loops that don't do
//! anything complicated, the principle demonstrated above fully applies when
//! using them.
//!
//! Contrast that with a closure-based approach:
//!
//! ``` ignore
//! let mut thing_being_idle = Thing::new();
//!
//! loop {
//!     let closure = || {
//!         // Since `do_stuff` takes ownership of the idle thing, the whole
//!         // closure takes ownership. We'll actually get a compiler error
//!         // here, as the compiler doesn't really understand that the closure
//!         // also gives this ownership back. See comment below.
//!         let thing_doing_stuff = thing_being_idle.do_stuff();
//!
//!         // stuff is being done
//!
//!         // Like in the example above, we try to give ownership back, so we
//!         // can use the variable again in the next loop iteration. However,
//!         // the compiler doesn't seem to have a concept of closures giving
//!         // back ownership, so it won't understand this, and the whole
//!         // example will not compile.
//!         thing_being_idle = thing_doing_stuff.finish_doing_stuff();
//!     };
//!
//!     closure();
//! }
//!
//!
//! # struct Thing<State>(State);
//! #
//! # impl Thing<BeingIdle> {
//! #     fn new() -> Self {
//! #         Thing(BeingIdle)
//! #     }
//! #
//! #     fn do_stuff(self) -> Thing<DoingStuff> {
//! #         // Start doing the important stuff
//! #         // ...
//! #
//! #         Thing(DoingStuff)
//! #     }
//! # }
//! #
//! # impl Thing<DoingStuff> {
//! #     fn finish_doing_stuff(self) -> Thing<BeingIdle> {
//! #         // Finish doing the important stuff
//! #         // ...
//! #
//! #         Thing(BeingIdle)
//! #     }
//! # }
//! #
//! # struct BeingIdle;
//! # struct DoingStuff;
//! ```
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

            // Make sure the timer has the right type. If it hasn't, the user
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
/// - A pseudo-closure that will be called every time the operation succeeds
///   This pseudo-closure is expected to take an argument of type `T`. The
///   return value is ignored.
/// - A pseudo-closure that will be called every time the operation fails
///   This pseudo-closure is expected to take an argument of type `E`. The
///   return value is ignored.
///
/// `repeat_timeout!` will keep repeating the operation until the timer runs
/// out, no matter whether it suceeds or fails.
///
/// It uses a `loop` to do that, which is `break`s from when the timer runs out.
/// Any of the expressions passed into the macro, the main expression, as well
/// as the two pseudo-closures, can employ `break` and `continue` to manipulate
/// that loop.
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
///         // The macro will keep evaluating this expression repeatedly until
///         // the timer times out.
///         //
///         // We can do anything that returns `Result` here. For this simple
///         // example, we just return `Ok`.
///         Ok(())
///
///         // We could also return an error.
///         // Err("This is an error")
///     },
///     // Here's a pseudo-closure with an argument in parentheses, which we can
///     // name freely, followed by an expression whose return value is ignored.
///     (result) {
///         // The macro will evaluate this expression, if the main expression
///         // above returns `Ok`. `result`, which we've named in the
///         // parentheses above, will be whatever the contents of the `Ok` are.
///         let result: () = result;
///     };
///     (error) {
///         // will be called by the macro, if the expression returns `Err`
///         let error: &'static str = error;
///     };
/// );
/// ```
#[macro_export]
macro_rules! repeat_timeout {
    (
        $timer:expr,
        $op:expr,
        ($result:ident) $on_success:expr;
        ($error:ident) $on_error:expr;
    ) => {
        {
            use $crate::embedded_hal::prelude::*;

            // Make sure the timer has the right type. If it hasn't, the user
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
                        let $result = result;
                        $on_success;
                    }
                    Err(error) => {
                        let $error = error;
                        $on_error;
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
