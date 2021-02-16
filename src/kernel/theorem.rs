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

use crate::kernel::handle::Handle;

////////////////////////////////////////////////////////////////////////////////
// Theorems, proper.
////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Theorem {
    /// The hypotheses of the theorem, i.e. the set of propositions that must
    /// hold for the conclusion to also hold.  All elements of this list should
    /// be handles pointing-to propositions in the runtime state's term-table.
    /// Handles should be stored in ascending sorted order.
    hypotheses: Vec<Handle>,
    /// The conclusion of the theorem, which must be a handle pointing-to a
    /// proposition in the runtime state's term-table.
    conclusion: Handle,
}

impl Theorem {
    /// Creates a new theorem from a collection of hypotheses and a handle to a
    /// conclusion.  Hypotheses are sorted before constructing the theorem
    /// object and are checked to make sure they all point-to propositions.
    /// Similarly, it is assumed that `conclusion` also points-to a proposition.
    pub fn new<T>(hypotheses: T, conclusion: Handle) -> Self
    where
        T: Iterator<Item = Handle>,
    {
        let mut hypotheses: Vec<Handle> = hypotheses.collect();

        hypotheses.sort();
        hypotheses.dedup();

        Theorem {
            hypotheses,
            conclusion,
        }
    }

    /// Returns the handle to the theorem's conclusion.
    #[inline]
    pub fn conclusion(&self) -> &Handle {
        &self.conclusion
    }

    /// Returns the set of hypotheses of the theorem.
    #[inline]
    pub fn hypotheses(&self) -> &Vec<Handle> {
        &self.hypotheses
    }
}

////////////////////////////////////////////////////////////////////////////////
// Tests.
////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod test {
    use crate::kernel::{handle::PREALLOCATED_HANDLE_TERM_TRUE, theorem::Theorem};

    /// Tests that constructing then deconstructing a theorem object gets you
    /// back to where you started.
    #[test]
    pub fn theorem_test0() {
        let t = Theorem::new(vec![].iter().cloned(), PREALLOCATED_HANDLE_TERM_TRUE);

        assert_eq!(t.hypotheses(), &Vec::new());
        assert_eq!(t.conclusion(), &PREALLOCATED_HANDLE_TERM_TRUE);
    }
}
