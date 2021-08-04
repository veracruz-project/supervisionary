//! # Bindings to Supervisionary's constant ABI
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

use crate::handle::{tags, Handle};
use crate::RawHandle;
use std::marker::PhantomData;

////////////////////////////////////////////////////////////////////////////////
// Pre-allocated constant-related handles.
////////////////////////////////////////////////////////////////////////////////

/// A pre-allocated handle used to refer to the truth constant.
pub const PREALLOCATED_HANDLE_CONSTANT_TRUE: Handle<tags::Constant> =
    Handle::new(10usize, PhantomData);
/// A pre-allocated handle used to refer to the falsity constant.
pub const PREALLOCATED_HANDLE_CONSTANT_FALSE: Handle<tags::Constant> =
    Handle::new(11usize, PhantomData);
/// A pre-allocated handle used to refer to the negation constant.
pub const PREALLOCATED_HANDLE_CONSTANT_NEGATION: Handle<tags::Constant> =
    Handle::new(12usize, PhantomData);
/// A pre-allocated handle used to refer to the binary conjunction connective.
pub const PREALLOCATED_HANDLE_CONSTANT_CONJUNCTION: Handle<tags::Constant> =
    Handle::new(13usize, PhantomData);
/// A pre-allocated handle used to refer to the binary disjunction connective.
pub const PREALLOCATED_HANDLE_CONSTANT_DISJUNCTION: Handle<tags::Constant> =
    Handle::new(14usize, PhantomData);
/// A pre-allocated handle used to refer to the binary implication connective.
pub const PREALLOCATED_HANDLE_CONSTANT_IMPLICATION: Handle<tags::Constant> =
    Handle::new(15usize, PhantomData);
/// A pre-allocated handle used to refer to the universal quantifier constant.
pub const PREALLOCATED_HANDLE_CONSTANT_FORALL: Handle<tags::Constant> =
    Handle::new(16usize, PhantomData);
/// A pre-allocated handle used to refer to the existential quantifier constant.
pub const PREALLOCATED_HANDLE_CONSTANT_EXISTS: Handle<tags::Constant> =
    Handle::new(17usize, PhantomData);
/// A pre-allocated handle used to refer to the equality constant.
pub const PREALLOCATED_HANDLE_CONSTANT_EQUALITY: Handle<tags::Constant> =
    Handle::new(18usize, PhantomData);

////////////////////////////////////////////////////////////////////////////////
// ABI bindings.
////////////////////////////////////////////////////////////////////////////////

extern "C" {
    /// Raw ABI binding to the `__constant_is_registered` function.
    fn __constant_is_registered(handle: RawHandle) -> i32;
    /// Raw ABI binding to the `__constant_resolve` function.
    fn __constant_resolve(handle: RawHandle, result: *mut RawHandle) -> i32;
    /// Raw ABI binding to the `__constant_register` function.
    fn __constant_register(
        type_handle: RawHandle,
        result: *mut RawHandle,
    ) -> i32;
}

#[inline]
pub fn constant_is_registered<H>(handle: H) -> bool
where
    H: Into<Handle<tags::Constant>>,
{
    let result = unsafe { __constant_is_registered(*handle.into() as u64) };

    result == 0
}
