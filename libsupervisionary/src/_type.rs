//! # Bindings to Supervisionary's type ABI
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

use crate::error_code::ErrorCode;
use crate::{
    handle::{tags, Handle},
    Name, RawHandle,
};
use std::marker::PhantomData;

////////////////////////////////////////////////////////////////////////////////
// Pre-allocated type-related handles.
////////////////////////////////////////////////////////////////////////////////

/// A pre-allocated handle used to refer to the type-variable `A`.
pub const PREALLOCATED_HANDLE_TYPE_ALPHA: Handle<tags::Type> =
    Handle::new(2usize, PhantomData);
/// A pre-allocated handle used to refer to the type-variable `B`.
pub const PREALLOCATED_HANDLE_TYPE_BETA: Handle<tags::Type> =
    Handle::new(3usize, PhantomData);
/// A pre-allocated handle used to refer to the `Prop` type.
pub const PREALLOCATED_HANDLE_TYPE_PROP: Handle<tags::Type> =
    Handle::new(4usize, PhantomData);
/// A pre-allocated handle used to refer to the type of unary predicates.
pub const PREALLOCATED_HANDLE_TYPE_UNARY_PREDICATE: Handle<tags::Type> =
    Handle::new(5usize, PhantomData);
/// A pre-allocated handle used to refer to the type of binary predicates.
pub const PREALLOCATED_HANDLE_TYPE_BINARY_PREDICATE: Handle<tags::Type> =
    Handle::new(6usize, PhantomData);
/// A pre-allocated handle used to refer to the type of unary connectives.
pub const PREALLOCATED_HANDLE_TYPE_UNARY_CONNECTIVE: Handle<tags::Type> =
    Handle::new(7usize, PhantomData);
/// A pre-allocated handle used to refer to the type of binary connectives.
pub const PREALLOCATED_HANDLE_TYPE_BINARY_CONNECTIVE: Handle<tags::Type> =
    Handle::new(8usize, PhantomData);
/// A pre-allocated handle used to refer to the type of polymorphic quantifiers.
pub const PREALLOCATED_HANDLE_TYPE_QUANTIFIER: Handle<tags::Type> =
    Handle::new(9usize, PhantomData);

////////////////////////////////////////////////////////////////////////////////
// ABI bindings.
////////////////////////////////////////////////////////////////////////////////

extern "C" {
    fn __type_is_registered(handle: RawHandle) -> bool;
    fn __type_register_variable(name: Name) -> u64;
    fn __type_register_combination(
        type_former_handle: RawHandle,
        argument_base: *const RawHandle,
        argument_length: u64,
        result: *mut RawHandle,
    ) -> i32;
    fn __type_register_function(
        domain_handle: RawHandle,
        range_handle: RawHandle,
        result: *mut RawHandle,
    ) -> i32;
    fn __type_split_variable(handle: RawHandle, result: *mut Name) -> i32;
    fn __type_split_combination(
        handle: RawHandle,
        argument_base: *mut RawHandle,
        argument_length: *mut u64,
    ) -> i32;
    fn __type_split_function(
        handle: RawHandle,
        domain_handle: *mut RawHandle,
        range_handle: *mut RawHandle,
    ) -> i32;
    fn __type_test_variable(handle: RawHandle) -> bool;
    fn __type_test_combination(handle: RawHandle) -> bool;
    fn __type_test_function(handle: RawHandle) -> bool;
    fn __type_variables(
        handle: RawHandle,
        result_base: *mut Name,
        result_length: u64,
    ) -> i32;
    fn __type_substitute(
        handle: RawHandle,
        domain_base: *const Name,
        domain_length: u64,
        range_base: *const RawHandle,
        range_length: u64,
        result: *mut RawHandle,
    ) -> i32;
}

/// Returns `true` iff `handle` points-to a registered type in the kernel's
/// heap.
#[inline]
pub fn type_is_registered<H>(handle: H) -> bool
where
    H: AsRef<Handle<tags::Type>>,
{
    unsafe { __type_is_registered(*handle.as_ref().clone() as u64) }
}

/// Allocates a new type-variable with a given `name`.  Note that this function
/// enforces maximal sharing in the kernel: allocating a second type-variable
/// with the same name as a previously-allocated variable returns the handle of
/// the previously-allocated variable.
#[inline]
pub fn type_register_variable<N>(name: N) -> Handle<tags::Type>
where
    N: Into<Name>,
{
    let raw_handle: u64 = unsafe { __type_register_variable(name.into()) };

    Handle::new(raw_handle as usize, PhantomData)
}

/// Allocates a new type combination, wherein a type-former `type_former` is
/// applied to a list of argument types, `arguments`.
///
/// # Errors
///
/// Returns `ErrorCode::NoSuchTypeFormer` if `type_former` does not point-to an
/// allocated type-former in the kernel's heaps.
///
/// Returns `ErrorCode::MismatchedArity` if the length of `arguments` does not
/// match the registered arity of `type_former`.
///
/// Returns `ErrorCode::NoSuchType` if any of the handles in `arguments` does
/// not point-to an allocated type in the kernel's heaps.
pub fn type_register_combination<T, A>(
    _type_former: T,
    _arguments: Vec<A>,
) -> Result<Handle<tags::Type>, ErrorCode>
where
    T: Into<Handle<tags::TypeFormer>>,
    A: Into<Handle<tags::Type>>,
{
    unimplemented!()
}
