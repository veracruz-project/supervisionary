//! # Fresh name generation
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
// Miscellaneous material.
////////////////////////////////////////////////////////////////////////////////

use crate::kernel::kernel_panic::kernel_info;

////////////////////////////////////////////////////////////////////////////////
// Names and related material.
////////////////////////////////////////////////////////////////////////////////

/// We use `u64` values to represent variable names.
pub type Name = u64;

/// Fresh name generation, for e.g. implementing the capture-avoiding
/// substitution action.  Finds a name that is not contained in the `avoid` set
/// of names.
fn fresh<T>(mut avoid: T) -> Name
where
    T: Iterator<Item = Name>,
{
    let mut counter = 0u64;

    loop {
        if avoid.any(|x| x == counter) {
            counter += 1;
        } else {
            kernel_info(format!("Fresh name generated: {}.", counter));
            return counter;
        }
    }
}

////////////////////////////////////////////////////////////////////////////////
// Tests.
////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod test {
    use crate::kernel::name::fresh;

    /// Tests that fresh-name generation is indeed fresh.
    #[test]
    pub fn name_test0() {
        let n = fresh(0u64..100u64);

        assert!(!(0u64..100u64).contains(&n));
    }
}
