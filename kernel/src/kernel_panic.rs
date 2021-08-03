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

////////////////////////////////////////////////////////////////////////////////
// Kernel panic messages.
////////////////////////////////////////////////////////////////////////////////

/// Error message produced during a kernel panic due to the kernel running out
/// of fresh handles.
pub const HANDLE_EXHAUST_ERROR: &str =
    "Kernel invariant failed: handles have been exhausted";

/// Error message produced during a kernel panic due to the kernel encountering
/// a registered kernel-object with a dangling handle.

pub const DANGLING_HANDLE_ERROR: &str =
    "Kernel invariant failed: dangling handle.";

/// Error message produced during a kernel panic due to the kernel failing to
/// build a kernel primitive.
pub const PRIMITIVE_CONSTRUCTION_ERROR: &str =
    "Kernel invariant failed: failed to construct a kernel primitive.";
