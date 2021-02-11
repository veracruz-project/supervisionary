//! # Integration with WASMI
//!
//! *Note that this is trusted code.*
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

pub mod _type;
/// Error codes used to indicate errors across the ABI boundary.
pub mod error_code;
/// Handles used to uniquely identify kernel objects.  Various pre-allocated
/// handles are also defined in this module, used to refer to primitive kernel
/// objects.
pub mod handle;
pub mod host_interface;
pub mod kernel_panic;
pub mod name;
pub mod runtime_state;
pub mod term;
pub mod theorem;
