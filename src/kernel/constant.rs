//! # Term constants
//!
//! Constants are registered with the kernel with a given type.  Whenever a
//! constant is constructed, we are entitled to specialize the type.  For
//! example, whilst the `Nil` constructor for lists may be registered with the
//! kernel at type `List A`, where `A` is a type-variable, we are entitled to
//! construct a `Nil` constructor at type `List Prop`, another at type
//! `List Nat`, and so on and so forth.  As a result, (polymorphic) HOL
//! constants are better thought of as introducing a family of related
//! constants.
//!
//! This module implements the kernel's constant table, associating handles to
//! types (handles therefore name constants).
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

use crate::kernel::error_code::ErrorCode;
use crate::kernel::{
    _type::Type,
    handle::{issue_handle, Handle},
};
use lazy_static::lazy_static;
use std::{collections::HashMap, sync::Mutex};

////////////////////////////////////////////////////////////////////////////////
// Miscellaneous material.
////////////////////////////////////////////////////////////////////////////////

/// The error message used when panicking if the lock on the constant table
/// cannot be obtained.
const TABLE_LOCK_ERROR: &str = "Failed to obtain lock on constant table.";

////////////////////////////////////////////////////////////////////////////////
// The type-former table.
////////////////////////////////////////////////////////////////////////////////

lazy_static! {
    static ref CONSTANT_TABLE: Mutex<HashMap<Handle, Type>> = {
        let mut table = HashMap::new();

        Mutex::new(table)
    };
}

/// Registers a new constant in the constant table, with a given type, `tau`.
/// Returns the handle of the newly-registered constant.
///
/// Returns `Err(ErrorCode::TypeNotWellformed)` if `tau` is not well-formed,
/// containing dangling pointers.
///
/// Will **panic** if a lock on the constant-table cannot be obtained.
pub fn register_constant(tau: Type) -> Result<Handle, ErrorCode> {
    let mut table = CONSTANT_TABLE.lock().expect(TABLE_LOCK_ERROR);

    if !tau.is_well_formed() {
        return Err(ErrorCode::TypeNotWellformed);
    }

    for (handle, registered) in table.iter() {
        if registered == &tau {
            return Ok(*handle);
        }
    }

    let fresh = issue_handle();

    table.insert(fresh, tau);

    Ok(fresh)
}

/// Returns `Some(tau)` iff a constant associated with the handle is registered
/// in the constant-table with type `tau`.
///
/// Will **panic** if a lock on the type-former table cannot be obtained.
#[inline]
pub fn constant_type(handle: &Handle) -> Option<Type> {
    CONSTANT_TABLE
        .lock()
        .expect(TABLE_LOCK_ERROR)
        .get(handle)
        .map(|tau| tau.clone())
}

/// Returns `true` iff a constant is associated with the handle is registered
/// in the constant-table.
///
/// Will **panic** if a lock on the constant-table cannot be obtained.
#[inline]
pub fn is_constant_registered(handle: &Handle) -> bool {
    constant_type(handle).is_some()
}
