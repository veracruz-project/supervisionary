//! # libsupervisionary
//!
//! Rust language support for interacting with the Supervisionary kernel.  This
//! library provides a binding to the kernel's host-call interface, with some
//! minor abstractions built on top.
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

#![feature(const_fn_trait_bound)]
#![feature(shrink_to)]

/// Bindings to the Supervisionary kernel's type ABI.
pub mod _type;
/// Bindings to the Supervisionary kernel's constant ABI.
pub mod constant;
/// Kernel error codes.  Note that the material in this module must exactly
/// match the definition of error code used in the kernel.
pub mod error_code;
/// The kernel handle type, and some primitive handles associated with the
/// Supervisionary ABI.
pub mod handle;
/// Bindings to the Supervisionary kernel's term ABI.
pub mod term;
/// Bindings to the Supervisionary kernel's theorem ABI.
pub mod theorem;
/// Bindings to the Supervisionary kernel's type-former ABI.
pub mod type_former;

/// The kernel type of names.
pub type Name = u64;
/// The kernel type of arities.
pub type Arity = u64;
/// The "raw" representation of handles expected by the kernel.
pub(crate) type RawHandle = u64;
/// The "raw" representation of kernel error modes.
pub(crate) type RawKernelFailureMode = i32;
