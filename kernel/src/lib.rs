//! # The Supervisionary kernel
//!
//! Supervisionary is a proof-checker for HOL.  The novelty of this
//! proof-checker lies in how it protects itself from untrusted code: unlike
//! other implementations of HOL, such as Isabelle/HOL and HOL4, where the
//! kernel is isolated using linguistic mechanisms, such as programming language
//! module boundaries and type-abstraction, Supervisionary uses machine-oriented
//! notions of isolation more typically-found in operating systems, namely
//! privilege levels and different memory spaces.
//!
//! Note: this library defines the Supervisionary kernel and is therefore
//! trusted code.
//!
//! # Authors
//!
//! [Dominic Mulligan], Systems Research Group, [Arm Research] Cambridge.
//! [Nick Spinale], Systems Research Group, [Arm Research] Cambridge.
//!
//! # Copyright
//!
//! Copyright (c) Arm Limited, 2021.  All rights reserved (r).  Please see the
//! `LICENSE.markdown` file in the *Supervisionary* root directory for licensing
//! information.
//!
//! [Dominic Mulligan]: https://dominic-mulligan.co.uk
//! [Nick Spinale]: https://nickspinale.com
//! [Arm Research]: http://www.arm.com/research

pub mod _type;
pub mod error_code;
pub mod handle;
pub mod kernel_panic;
pub mod name;
pub mod runtime_state;
pub mod term;
pub mod theorem;
