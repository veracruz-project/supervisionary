//! # Bindings to Supervisionary's type-former ABI
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

use crate::{
    error_code::ErrorCode,
    handle::{tags, Handle},
    Arity, RawHandle, RawKernelFailureMode,
};
use std::{convert::TryFrom, marker::PhantomData};

////////////////////////////////////////////////////////////////////////////////
// Pre-allocated type-former handles.
////////////////////////////////////////////////////////////////////////////////

/// A pre-allocated handle used to refer to the `Prop` type-former.
pub const PREALLOCATED_HANDLE_TYPE_FORMER_PROP: Handle<tags::TypeFormer> =
    Handle::new(0usize, PhantomData);
/// A pre-allocated handle used to refer to the function-space type-former.
pub const PREALLOCATED_HANDLE_TYPE_FORMER_ARROW: Handle<tags::TypeFormer> =
    Handle::new(1usize, PhantomData);

////////////////////////////////////////////////////////////////////////////////
// ABI bindings.
////////////////////////////////////////////////////////////////////////////////

extern "C" {
    /// Raw ABI binding to the `__type_former_register` function.
    fn __type_former_register(handle: RawHandle) -> usize;
    /// Raw ABI binding to the `__type_former_is_registered` function.
    fn __type_former_is_registered(handle: RawHandle) -> bool;
    /// Raw ABI binding to the `__type_former_resolve` function.
    fn __type_former_resolve(
        handle: RawHandle,
        out: *mut usize,
    ) -> RawKernelFailureMode;
}

/// Registers a new type-former with a given `arity`.  Returns the handle to the
/// new type-former.  Note that this function is generative, in the sense that
/// registering two type-formers with the same arity results in two different
/// type-formers.
pub fn type_former_register<T>(arity: T) -> Handle<tags::TypeFormer>
where
    T: Into<usize>,
{
    let handle = unsafe { __type_former_register(arity.into()) };

    Handle::new(handle, PhantomData)
}

/// Returns `true` iff `handle` points-to a registered type-former in the
/// kernel's heap.
pub fn type_former_is_registered(handle: Handle<tags::TypeFormer>) -> bool {
    unsafe { __type_former_is_registered(*handle) }
}

/// Returns the arity of the type-former pointed-to by `handle` in the kernel's
/// heap, if any.
pub fn type_former_resolve(
    handle: Handle<tags::TypeFormer>,
) -> Result<Arity, ErrorCode> {
    let mut arity: Arity = 0usize;

    let result =
        unsafe { __type_former_resolve(*handle, &mut arity as *mut usize) };

    if result == 0 {
        Ok(arity)
    } else {
        Err(ErrorCode::try_from(result).unwrap())
    }
}
