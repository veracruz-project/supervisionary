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

/// The default stem for fresh name generation if not explicitly over-ridden by
/// the caller.
const FRESH_NAME_STEM: &str = "f";

////////////////////////////////////////////////////////////////////////////////
// Names and related material.
////////////////////////////////////////////////////////////////////////////////

/// We use Strings to represent variable names.
pub type Name = String;

/// Fresh name generation, for e.g. implementing the capture-avoiding
/// substitution action.  Finds a name that is not contained in the `avoid` set
/// of names. If `base` is `Some(b)` for a name `b` then `b` is used as the stem
/// of the freshly-generated name, otherwise a default is used.
fn fresh<T, U>(base: Option<U>, mut avoid: T) -> Name
where
    T: Iterator<Item = Name>,
    U: Into<Name>,
{
    let mut counter = 0_usize;

    let base = base
        .map(|b| b.into())
        .unwrap_or(String::from(FRESH_NAME_STEM));

    loop {
        let generated = format!("{}{}", base, counter);

        if avoid.any(|x| x == generated) {
            counter += 1;
        } else {
            return generated;
        }
    }
}
