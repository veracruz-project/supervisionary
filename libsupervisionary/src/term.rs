//! # Bindings to Supervisionary's term ABI
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
    Name, RawHandle,
};
use std::marker::PhantomData;

////////////////////////////////////////////////////////////////////////////////
// Pre-allocated term-related handles.
////////////////////////////////////////////////////////////////////////////////

/// A pre-allocated handle used to refer to the truth term, the truth constant
/// lifted into a term.
pub const PREALLOCATED_HANDLE_TERM_TRUE: Handle<tags::Term> =
    Handle::new(19usize, PhantomData);
/// A pre-allocated handle used to refer to the falsity term, the falsity
/// constant lifted into a term.
pub const PREALLOCATED_HANDLE_TERM_FALSE: Handle<tags::Term> =
    Handle::new(20usize, PhantomData);
/// A pre-allocated handle used to refer to the negation term, the negation
/// constant lifted into a term.
pub const PREALLOCATED_HANDLE_TERM_NEGATION: Handle<tags::Term> =
    Handle::new(21usize, PhantomData);
/// A pre-allocated handle used to refer to the conjunction term, the
/// conjunction constant lifted into a term.
pub const PREALLOCATED_HANDLE_TERM_CONJUNCTION: Handle<tags::Term> =
    Handle::new(22usize, PhantomData);
/// A pre-allocated handle used to refer to the disjunction term, the
/// disjunction constant lifted into a term.
pub const PREALLOCATED_HANDLE_TERM_DISJUNCTION: Handle<tags::Term> =
    Handle::new(23usize, PhantomData);
/// A pre-allocated handle used to refer to the implication term, the
/// implication constant lifted into a term.
pub const PREALLOCATED_HANDLE_TERM_IMPLICATION: Handle<tags::Term> =
    Handle::new(24usize, PhantomData);
/// A pre-allocated handle used to refer to the equality term, the equality
/// constant lifted into a term.
pub const PREALLOCATED_HANDLE_TERM_EQUALITY: Handle<tags::Term> =
    Handle::new(25usize, PhantomData);
/// A pre-allocated handle used to refer to the universal quantifier term, the
/// universal quantifier constant lifted into a term.
pub const PREALLOCATED_HANDLE_TERM_FORALL: Handle<tags::Term> =
    Handle::new(26usize, PhantomData);
/// A pre-allocated handle used to refer to the existential quantifier term, the
/// existential quantifier constant lifted into a term.
pub const PREALLOCATED_HANDLE_TERM_EXISTS: Handle<tags::Term> =
    Handle::new(27usize, PhantomData);

////////////////////////////////////////////////////////////////////////////////
// ABI bindings.
////////////////////////////////////////////////////////////////////////////////

extern "C" {
    /// Raw ABI binding to the `__term_is_registered` function.
    fn __term_is_registered(handle: RawHandle) -> bool;
    /// Raw ABI binding to the `__term_register_variable` function.
    fn __term_register_variable(
        name: Name,
        type_handle: RawHandle,
        result: *mut RawHandle,
    ) -> i32;
    /// Raw ABI binding to the `__term_register_constant` function.
    fn __term_register_constant(
        constant_handle: RawHandle,
        domain_base: *const Name,
        domain_length: u64,
        range_base: *mut RawHandle,
        range_length: u64,
        result: *mut RawHandle,
    ) -> i64;
    /// Raw ABI binding to the `__term_register_application` function.
    fn __term_register_application(
        left_handle: RawHandle,
        right_handle: RawHandle,
        result: *mut RawHandle,
    ) -> i32;
    /// Raw ABI binding to the `__term_register_lambda` function.
    fn __term_register_lambda(
        bound_name: Name,
        type_handle: RawHandle,
        body_handle: RawHandle,
        result: *mut RawHandle,
    ) -> i32;
    /// Raw ABI binding to the `__term_register_negation` function.
    fn __term_register_negation(
        body_handle: RawHandle,
        result: *mut RawHandle,
    ) -> i32;
    /// Raw ABI binding to the `__term_register_conjunction` function.
    fn __term_register_conjunction(
        left_handle: RawHandle,
        right_handle: RawHandle,
        result: *mut RawHandle,
    ) -> i32;
    /// Raw ABI binding to the `__term_register_disjunction` function.
    fn __term_register_disjunction(
        left_handle: RawHandle,
        right_handle: RawHandle,
        result: *mut RawHandle,
    ) -> i32;
    /// Raw ABI binding to the `__term_register_implication` function.
    fn __term_register_implication(
        left_handle: RawHandle,
        right_handle: RawHandle,
        result: *mut RawHandle,
    ) -> i32;
    /// Raw ABI binding to the `__term_register_equality` function.
    fn __term_register_equality(
        left_handle: RawHandle,
        right_handle: RawHandle,
        result: *mut RawHandle,
    ) -> i32;
    /// Raw ABI binding to the `__term_register_forall` function.
    fn __term_register_forall(
        bound_name: Name,
        type_handle: RawHandle,
        body_handle: RawHandle,
        result: *mut RawHandle,
    ) -> i32;
    /// Raw ABI binding to the `__term_register_exists` function.
    fn __term_register_exists(
        bound_name: Name,
        type_handle: RawHandle,
        body_handle: RawHandle,
        result: *mut RawHandle,
    ) -> i32;
    /// Raw ABI binding to the `__term_split_variable` function.
    fn __term_split_variable(
        term_handle: RawHandle,
        result_name: *mut Name,
        result_type: *mut RawHandle,
    ) -> i32;
    /// Raw ABI binding to the `__term_split_constant` function.
    fn __term_split_constant(
        term_handle: RawHandle,
        result_domain_base: *mut Name,
        result_domain_length: *mut u64,
        result_range_base: *mut RawHandle,
        result_range_length: *mut u64,
    ) -> i32;
    /// Raw ABI binding to the `__term_split_application` function.
    fn __term_split_application(
        term_handle: RawHandle,
        result_left: *mut RawHandle,
        result_right: *mut RawHandle,
    ) -> i32;
    /// Raw ABI binding to the `__term_split_lambda` function.
    fn __term_split_lambda(
        term_handle: RawHandle,
        result_name: *mut Name,
        result_type: *mut RawHandle,
        result_body: *mut RawHandle,
    ) -> i32;
    /// Raw ABI binding to the `__term_split_negation` function.
    fn __term_split_negation(
        term_handle: RawHandle,
        result_body: *mut RawHandle,
    ) -> i32;
    /// Raw ABI binding to the `__term_split_conjunction` function.
    fn __term_split_conjunction(
        term_handle: RawHandle,
        result_left: *mut RawHandle,
        result_right: *mut RawHandle,
    ) -> i32;
    /// Raw ABI binding to the `__term_split_disjunction` function.
    fn __term_split_disjunction(
        term_handle: RawHandle,
        result_left: *mut RawHandle,
        result_right: *mut RawHandle,
    ) -> i32;
    /// Raw ABI binding to the `__term_split_implication` function.
    fn __term_split_implication(
        term_handle: RawHandle,
        result_left: *mut RawHandle,
        result_right: *mut RawHandle,
    ) -> i32;
    /// Raw ABI binding to the `__term_split_equality` function.
    fn __term_split_equality(
        term_handle: RawHandle,
        result_left: *mut RawHandle,
        result_right: *mut RawHandle,
    ) -> i32;
    /// Raw ABI binding to the `__term_split_forall` function.
    fn __term_split_forall(
        term_handle: RawHandle,
        result_name: *mut Name,
        result_type: *mut RawHandle,
        result_body: *mut RawHandle,
    ) -> i32;
    /// Raw ABI binding to the `__term_split_exists` function.
    fn __term_split_exists(
        term_handle: RawHandle,
        result_name: *mut Name,
        result_type: *mut RawHandle,
        result_body: *mut RawHandle,
    ) -> i32;
    /// Raw ABI binding to the `__term_test_variable` function.
    fn __term_test_variable(term_handle: RawHandle, result: *mut u32) -> i32;
    /// Raw ABI binding to the `__term_test_constant` function.
    fn __term_test_constant(term_handle: RawHandle, result: *mut u32) -> i32;
    /// Raw ABI binding to the `__term_test_application` function.
    fn __term_test_application(term_handle: RawHandle, result: *mut u32)
        -> i32;
    /// Raw ABI binding to the `__term_test_lambda` function.
    fn __term_test_lambda(term_handle: RawHandle, result: *mut u32) -> i32;
    /// Raw ABI binding to the `__term_test_negation` function.
    fn __term_test_negation(term_handle: RawHandle, result: *mut u32) -> i32;
    /// Raw ABI binding to the `__term_test_conjunction` function.
    fn __term_test_conjunction(term_handle: RawHandle, result: *mut u32)
        -> i32;
    /// Raw ABI binding to the `__term_test_disjunction` function.
    fn __term_test_disjunction(term_handle: RawHandle, result: *mut u32)
        -> i32;
    /// Raw ABI binding to the `__term_test_implication` function.
    fn __term_test_implication(term_handle: RawHandle, result: *mut u32)
        -> i32;
    /// Raw ABI binding to the `__term_test_equality` function.
    fn __term_test_equality(term_handle: RawHandle, result: *mut u32) -> i32;
    /// Raw ABI binding to the `__term_test_forall` function.
    fn __term_test_forall(term_handle: RawHandle, result: *mut u32) -> i32;
    /// Raw ABI binding to the `__term_test_exists` function.
    fn __term_test_exists(term_handle: RawHandle, result: *mut u32) -> i32;
    /// Raw ABI binding to the `__term_size` function.
    fn __term_size(handle: RawHandle, result: *mut u64) -> i32;
    /// Raw ABI binding to the `__term_free_variables` function.
    fn __term_free_variables(
        term_handle: RawHandle,
        result_name_base: *mut Name,
        result_name_length: u64,
        result_type_base: *mut RawHandle,
        result_type_length: u64,
    ) -> i32;
    /// Raw ABI binding to the `__term_free_type_variables` function.
    fn __term_free_type_variables(
        term_handle: RawHandle,
        result_type_base: *mut RawHandle,
        result_type_length: u64,
    ) -> i32;
    /// Raw ABI binding to the `__term_substitution` function.
    fn __term_substitution(
        term_handle: RawHandle,
        domain_base: *const Name,
        domain_length: u64,
        range_base: *const RawHandle,
        range_length: u64,
        result: *mut RawHandle,
    ) -> i32;
    /// Raw ABI binding to the `__term_type_substitution` function.
    fn __term_type_substitution(
        term_handle: RawHandle,
        domain_base: *const Name,
        domain_length: u64,
        range_base: *const RawHandle,
        range_length: u64,
        result: *mut RawHandle,
    ) -> i32;
    /// Raw ABI binding to the `__term_type_infer` function.
    fn __term_type_infer(term_handle: RawHandle, result: *mut RawHandle)
        -> i32;
    /// Raw ABI binding to the `__term_is_proposition` function.
    fn __term_is_proposition(term_handle: RawHandle, result: *mut u32) -> i32;
}
