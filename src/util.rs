//! Contains utility functions that are useful when working with the DW1000


use TIME_MAX;


/// Determines the duration between to time stamps
///
/// Expects two 40-bit system time stamps and returns the duration between the
/// two, taking potential overflow into account.
///
/// # Panics
///
/// Panics, if the time stamps passed don't fit within 40 bits.
pub fn duration_between(earlier: u64, later: u64) -> u64 {
    assert!(earlier <= TIME_MAX);
    assert!(later   <= TIME_MAX);

    if later >= earlier {
        later - earlier
    }
    else {
        TIME_MAX - earlier + later + 1
    }
}


/// Blocks on a non-blocking operation until a timer times out
///
/// Expects two arguments: A timer, and an expression that evaluates to
/// `nb::Result<T, E>` and returns `Result<T, TimeoutError<E>>`.
#[macro_export]
macro_rules! block_timeout {
    ($timer:expr, $op:expr) => {
        {
            use $crate::hal::prelude::TimerExt;
            let timer: &mut $crate::hal::Timer<_> = $timer;

            loop {
                match timer.wait() {
                    Ok(()) =>
                        break Err($crate::util::TimeoutError::Timeout),
                    Err(nb::Error::WouldBlock) =>
                        (),
                    Err(_) =>
                        unreachable!(),
                }

                match $op {
                    Ok(result) =>
                        break Ok(result),
                    Err(nb::Error::WouldBlock) =>
                        (),
                    Err(nb::Error::Other(error)) =>
                        break Err($crate::util::TimeoutError::Other(error)),
                }
            }
        }
    }
}


/// An error that can be a timeout or another error
#[derive(Debug)]
pub enum TimeoutError<T> {
    /// The operation timed out
    Timeout,

    /// Another error occured
    Other(T),
}
