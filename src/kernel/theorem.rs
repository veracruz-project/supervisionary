//! # Theorems
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

use crate::wasmi::handle::Handle;

////////////////////////////////////////////////////////////////////////////////
// Theorems, proper.
////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Theorem {
    /// The hypotheses of the theorem, i.e. the set of propositions that must
    /// hold for the conclusion to also hold.
    hypotheses: Vec<Handle>,
    /// The conclusion of the theorem, which must always be a proposition.
    conclusion: Handle,
}
