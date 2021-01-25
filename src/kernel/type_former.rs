//! # Type-formers
//!
//! Type-formers in HOL are essentially functions that take types as arguments
//! and create new types.  HOL restricts types so that type-formers are always
//! fully applied, and we therefore need to have some way of recording the
//! declared *arity*, that is, the number of arguments that it is expecting,
//! within the kernel.  This is the purpose of this module, which allows
//! prover-space code to register new type-formers with an arity prior to using
//! them to construct types.  HOL has two type-formers built in to the kernel,
//! `Prop`, the type-former of propositions with arity `0` and the
//! function-space type-former with arity `2`.  These are therefore
//! pre-registered in the type-former table.
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

use super::handle::{
    Handle, PREALLOCATED_HANDLE_TYPE_FORMER_ARROW, PREALLOCATED_HANDLE_TYPE_FORMER_PROP,
};
use crate::kernel::handle::issue_handle;
use lazy_static::lazy_static;
use std::{collections::HashMap, sync::Mutex};

////////////////////////////////////////////////////////////////////////////////
// Miscellaneous material.
////////////////////////////////////////////////////////////////////////////////

/// The error message used when panicking if the lock on the type-former table
/// cannot be obtained.
const TABLE_LOCK_ERROR: &str = "Failed to obtain lock on type-former table.";

////////////////////////////////////////////////////////////////////////////////
// The type-former table.
////////////////////////////////////////////////////////////////////////////////

lazy_static! {
    static ref TYPE_FORMER_TABLE: Mutex<HashMap<Handle, usize>> = {
        let mut table = HashMap::new();

        table.insert(PREALLOCATED_HANDLE_TYPE_FORMER_PROP, 0);
        table.insert(PREALLOCATED_HANDLE_TYPE_FORMER_ARROW, 2);

        Mutex::new(table)
    };
}

/// Registers a new type-former in the type-former table, with a given arity.
/// Returns the handle of the newly-registered type-former.
///
/// Will **panic** if a lock on the type-former table cannot be obtained.
pub fn register_type_former(arity: usize) -> Handle {
    let mut table = TYPE_FORMER_TABLE.lock().expect(TABLE_LOCK_ERROR);

    let handle = issue_handle();

    table.insert(handle.clone(), arity);

    handle
}

/// Returns `Some(arity)` iff a type-former associated with the handle is
/// registered in the type-former table with arity `arity`.
///
/// Will **panic** if a lock on the type-former table cannot be obtained.
#[inline]
pub fn type_former_arity(handle: &Handle) -> Option<usize> {
    TYPE_FORMER_TABLE
        .lock()
        .expect(TABLE_LOCK_ERROR)
        .get(handle)
        .map(|a| *a)
}

/// Returns `true` iff a type-former is associated with the handle is registered
/// in the type-former table.
///
/// Will **panic** if a lock on the type-former table cannot be obtained.
#[inline]
pub fn is_type_former_registered(handle: &Handle) -> bool {
    type_former_arity(handle).is_some()
}
