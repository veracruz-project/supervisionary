//! # Bindings to Supervisionary's theorem ABI
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
// ABI bindings.
////////////////////////////////////////////////////////////////////////////////

/* TODO: add bindings for existential inference rules */
extern "C" {
    fn __theorem_is_registered(theorem_handle: RawHandle) -> i32;
    fn __theorem_split_conclusion(
        theorem_handle: RawHandle,
        result: *mut RawHandle,
    ) -> i32;
    fn __theorem_split_hypotheses(
        theorem_handle: RawHandle,
        hypotheses_base: *mut RawHandle,
        hypotheses_length: u64,
    ) -> i32;
    fn __theorem_register_assumption(
        term_handle: RawHandle,
        hypotheses_base: *const RawHandle,
        hypotheses_length: u64,
        result: *mut RawHandle,
    ) -> i32;
    fn __theorem_register_reflexivity(
        term_handle: RawHandle,
        hypotheses_base: *const RawHandle,
        hypotheses_length: u64,
        result: *mut RawHandle,
    ) -> i32;
    fn __theorem_register_symmetry(
        theorem_handle: RawHandle,
        result: *mut RawHandle,
    ) -> i32;
    fn __theorem_register_transitivity(
        left_handle: RawHandle,
        right_handle: RawHandle,
        result: *mut RawHandle,
    ) -> i32;
    fn __theorem_register_application(
        left_handle: RawHandle,
        right_handle: RawHandle,
        result: *mut RawHandle,
    ) -> i32;
    fn __theorem_register_lambda(
        name: Name,
        type_handle: RawHandle,
        body_handle: RawHandle,
        result: *mut RawHandle,
    ) -> i32;
    fn __theorem_register_beta(
        term_handle: RawHandle,
        hypotheses_base: *const RawHandle,
        hypotheses_length: u64,
        result: *mut RawHandle,
    ) -> i32;
    fn __theorem_register_eta(
        term_handle: RawHandle,
        hypotheses_base: *const RawHandle,
        hypotheses_length: u64,
        result: *mut RawHandle,
    ) -> i32;
    fn __theorem_register_substitution(
        theorem_handle: RawHandle,
        domain_base: *const Name,
        domain_length: u64,
        range_base: *const RawHandle,
        range_length: u64,
        result: *mut RawHandle,
    ) -> i32;
    fn __theorem_register_type_substitution(
        theorem_handle: RawHandle,
        domain_base: *const Name,
        domain_length: u64,
        range_base: *const RawHandle,
        range_length: u64,
        result: *mut RawHandle,
    ) -> i32;
    fn __theorem_register_truth_introduction(
        hypotheses_base: *const RawHandle,
        hypotheses_length: u64,
        result: *mut RawHandle,
    ) -> i32;
    fn __theorem_register_falsity_elimination(
        theorem_handle: RawHandle,
        result: *mut RawHandle,
    ) -> i32;
    fn __theorem_register_conjunction_introduction(
        left_handle: RawHandle,
        right_handle: RawHandle,
        result: *mut RawHandle,
    ) -> i32;
    fn __theorem_register_conjunction_left_elimination(
        left_handle: RawHandle,
        result: *mut RawHandle,
    ) -> i32;
    fn __theorem_register_conjunction_right_elimination(
        left_handle: RawHandle,
        result: *mut RawHandle,
    ) -> i32;
    fn __theorem_register_disjunction_left_introduction(
        theorem_handle: RawHandle,
        term_handle: RawHandle,
        result: *mut RawHandle,
    ) -> i32;
    fn __theorem_register_disjunction_right_introduction(
        theorem_handle: RawHandle,
        term_handle: RawHandle,
        result: *mut RawHandle,
    ) -> i32;
    fn __theorem_register_disjunction_elimination(
        left_handle: RawHandle,
        mid_handle: RawHandle,
        right_handle: RawHandle,
        result: *mut RawHandle,
    ) -> i32;
    fn __theorem_register_negation_introduction(
        theorem_handle: RawHandle,
        term_handle: RawHandle,
        result: *mut RawHandle,
    ) -> i32;
    fn __theorem_register_negation_elimination(
        left_handle: RawHandle,
        right_handle: RawHandle,
        result: *mut RawHandle,
    ) -> i32;
    fn __theorem_register_implication_introduction(
        theorem_handle: RawHandle,
        term_handle: RawHandle,
        result: *mut RawHandle,
    ) -> i32;
    fn __theorem_register_implication_elimination(
        left_handle: RawHandle,
        right_handle: RawHandle,
        result: *mut RawHandle,
    ) -> i32;
    fn __theorem_register_iff_introduction(
        left_handle: RawHandle,
        right_handle: RawHandle,
        result: *mut RawHandle,
    ) -> i32;
    fn __theorem_register_iff_left_elimination(
        theorem_handle: RawHandle,
        result: *mut RawHandle,
    ) -> i32;
    fn __theorem_register_forall_introduction(
        theorem_handle: RawHandle,
        name: Name,
        type_handle: RawHandle,
        result: *mut RawHandle,
    ) -> i32;
    fn __theorem_register_forall_elimination(
        theorem_handle: RawHandle,
        term_handle: RawHandle,
        result: *mut RawHandle,
    ) -> i32;
}
