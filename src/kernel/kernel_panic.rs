//! # Kernel panic messages
//!
//! # Authors
//!
//! [Dominic Mulligan], Systems Research Group, [Arm Research] Cambridge.
//!
//! # Copyright
//!
//! Copyright (c) Arm Limited, 2021.  All rights reserved (r).  Please see the
//! `LICENSE.markdown` file in the *Supervisionary* root directory for licensing
//! information.
//!
//! [Dominic Mulligan]: https://dominic-mulligan.co.uk
//! [Arm Research]: http://www.arm.com/research

use log::{error, info};

////////////////////////////////////////////////////////////////////////////////
// Kernel logging.
////////////////////////////////////////////////////////////////////////////////

/// Kernel logging for information cases.
#[inline]
pub fn kernel_info<T>(message: T)
where
    T: AsRef<str>,
{
    info!("[KERNEL]: {}", message.as_ref())
}

/// Kernel logging for non-fatal error cases.
#[inline]
pub fn kernel_error<T>(message: T)
where
    T: AsRef<str>,
{
    error!("[KERNEL]: {}", message.as_ref())
}

/// Kernel logging for fatal cases: logs the provided error on the kernel's
/// error log, then panics with the same error message.
#[inline]
pub fn kernel_panic<T>(message: T)
where
    T: AsRef<str>,
{
    error!("[KERNEL]: {}", message.as_ref());
    panic!()
}

////////////////////////////////////////////////////////////////////////////////
// Kernel panic messages.
////////////////////////////////////////////////////////////////////////////////

/// Error message produced during a kernel panic due to the kernel running out
/// of fresh handles.
pub const HANDLE_EXHAUST_ERROR: &str = "Kernel invariant failed: handles have been exhausted";

/// Error message produced during a kernel panic due to the kernel encountering
/// a registered kernel-object with a dangling handle.

pub const DANGLING_HANDLE_ERROR: &str = "Kernel invariant failed: dangling handle.";

/// Error message produced during a kernel panic due to the kernel failing to
/// build a kernel primitive.
pub const PRIMITIVE_CONSTRUCTION_ERROR: &str =
    "Kernel invariant failed: failed to construct a kernel primitive.";
