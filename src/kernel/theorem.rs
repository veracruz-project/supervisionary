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

use crate::kernel::{
    error_code::ErrorCode,
    handle::{issue_handle, Handle},
    term::term,
};
use lazy_static::lazy_static;
use std::{collections::HashMap, sync::Mutex};

////////////////////////////////////////////////////////////////////////////////
// Miscellaneous material.
////////////////////////////////////////////////////////////////////////////////

/// The error message used when panicking if the lock on the theorem-table
/// cannot be obtained.
const TABLE_LOCK_ERROR: &str = "Failed to obtain lock on theorem-table.";

/// The error message used when panicking if a dangling handle is detected in a
/// theorem or other kernel object.
const DANGLING_HANDLE_ERROR: &str = "Kernel invariant failed: dangling handle.";

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

impl Theorem {
    /// Constructs a new theorem object from a list of handles pointing to
    /// assumptions, `hypotheses`, and a handle pointing to a conclusion.  Note
    /// that this function should not be exposed outside of this module: to
    /// ensure kernel soundness only the theorem construction functions defined
    /// in the rest of the file, corresponding to the inference rules of HOL,
    /// should be used.
    ///
    /// Returns `Err(ErrorCode::TermNotWellformed)` iff any of the hypotheses in
    /// `hypotheses` or the `conclusion` contain dangling handles.  Returns
    /// `Err(ErrorCode::NotAProposition)` iff any of the hypotheses in
    /// `hypotheses` or `conclusion` are not propositions.  Returns any one of
    /// several errors relating to type-inference iff any of the hypotheses in
    /// `hypotheses` or `conclusion` are ill-typed.
    fn new(hypotheses: Vec<Handle>, conclusion: Handle) -> Result<Self, ErrorCode> {
        for hypothesis in hypotheses.iter() {
            if let Some(hypothesis) = term(hypothesis) {
                if !hypothesis.is_well_formed() {
                    return Err(ErrorCode::TermNotWellformed);
                }

                if !hypothesis.is_proposition()? {
                    return Err(ErrorCode::NotAProposition);
                }
            } else {
                return Err(ErrorCode::NoSuchTermRegistered);
            }
        }

        if let Some(conclusion) = term(&conclusion) {
            if !conclusion.is_well_formed() {
                return Err(ErrorCode::TermNotWellformed);
            }

            if !conclusion.is_proposition()? {
                return Err(ErrorCode::NotAProposition);
            }
        } else {
            return Err(ErrorCode::NoSuchTermRegistered);
        }

        Ok(Theorem {
            hypotheses,
            conclusion,
        })
    }

    /// Returns `true` iff the theorem is well-formed, in the sense that it
    /// contains no dangling handles to terms, and all terms mentioned in the
    /// theorem are propositions.
    ///
    /// Note that is all "registration" functions for terms, types, constants,
    /// and so on, preserve the invariant that they only ever allow a kernel
    /// object to be registered if it is well-formed, then this check only needs
    /// to be shallow: only the handles actually appearing at the very "top" of
    /// the theorem need to be checked to ensure that the entire theorem is
    /// well-formed.
    pub fn is_well_formed(&self) -> bool {
        for hypothesis in self.hypotheses.iter() {
            if let Some(hypothesis) = term(hypothesis) {
                if !hypothesis.is_proposition() {
                    return false;
                }
            } else {
                return false;
            }
        }

        if let Some(conclusion) = term(&self.conclusion) {
            conclusion.is_proposition().is_ok()
        } else {
            false
        }
    }
}

////////////////////////////////////////////////////////////////////////////////
// The theorem-table.
////////////////////////////////////////////////////////////////////////////////

lazy_static! {
    static ref THEOREM_TABLE: Mutex<HashMap<Handle, Theorem>> = Mutex::new(HashMap::new());
}

pub fn register_theorem(thm: Theorem) -> Result<Handle, ErrorCode> {
    let mut table = THEOREM_TABLE.lock().expect(TABLE_LOCK_ERROR);

    if !thm.is_well_formed() {
        return Err(ErrorCode::TheoremNotWellformed);
    }

    for (handle, registered) in table.iter() {
        if registered == &thm {
            return Ok(*handle);
        }
    }

    let fresh = issue_handle();

    table.insert(fresh, thm);

    Ok(fresh)
}

pub fn theorem(handle: &Handle) -> Option<Theorem> {
    THEOREM_TABLE
        .lock()
        .expect(TABLE_LOCK_ERROR)
        .get(handle)
        .map(|t| t.clone())
}

#[inline]
pub fn is_theorem_registered(handle: &Handle) -> bool {
    theorem(handle).is_some()
}
