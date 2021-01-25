//! # Kernel object handles
//!
//! Kernel objects are manipulated only by the kernel, so prover-space code
//! needs some way of naming the object that should be manipulated by the
//! kernel.  In Supervisionary, we use *handles* for this purpose, which are
//! simply machine words suitable for passing across the kernel/prover-space ABI
//! boundary.
//!
//! This module contains material related to handles: specifically code for
//! issuing new, fresh handles, and also a series of pre-allocated handles that
//! are used to refer to primitive objects that are built-in to the kernel.
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

use lazy_static::lazy_static;
use std::sync::atomic::{AtomicUsize, Ordering};

////////////////////////////////////////////////////////////////////////////////
// Handles.
////////////////////////////////////////////////////////////////////////////////

/// We use the Rust `usize` type as our handle type.  Note that on modern 64-bit
/// systems this is implemented as a 64-bit unsigned integer.
pub type Handle = usize;

lazy_static! {
    static ref NEXT_AVAILABLE_HANDLE: AtomicUsize = AtomicUsize::new(28);
    static ref LAST_ISSUED_HANDLE: AtomicUsize = AtomicUsize::new(27);
}

/// Issues the next available fresh handle.  Note prover-space code must not
/// rely on fresh handles being issued incrementally.
///
/// This will **panic** if the kernel's handle counter overflows, breaking the
/// invariant that handles always uniquely identify any kernel object.
pub fn issue_handle() -> usize {
    let current = LAST_ISSUED_HANDLE.fetch_add(1, Ordering::SeqCst);
    let next = NEXT_AVAILABLE_HANDLE.fetch_add(1, Ordering::SeqCst);

    if next < current {
        panic!("The kernel has exhausted its supply of fresh handles.");
    } else {
        next
    }
}

/// Returns `true` iff the handle is a pre-allocated handle built into the
/// kernel.
#[inline]
pub fn is_preallocated(handle: Handle) -> bool {
    handle >= 0usize && handle < 28usize
}

////////////////////////////////////////////////////////////////////////////////
// Pre-allocated handles for kernel objects.
////////////////////////////////////////////////////////////////////////////////

/// A pre-allocated handle used to refer to the `Prop` type-former.
pub const PREALLOCATED_HANDLE_TYPE_FORMER_PROP: Handle = 0;
/// A pre-allocated handle used to refer to the function-space type-former.
pub const PREALLOCATED_HANDLE_TYPE_FORMER_ARROW: Handle = 1;
/// A pre-allocated handle used to refer to the type-variable `A`.
pub const PREALLOCATED_HANDLE_TYPE_ALPHA: Handle = 2;
/// A pre-allocated handle used to refer to the type-variable `B`.
pub const PREALLOCATED_HANDLE_TYPE_BETA: Handle = 3;
/// A pre-allocated handle used to refer to the `Prop` type.
pub const PREALLOCATED_HANDLE_TYPE_PROP: Handle = 4;
/// A pre-allocated handle used to refer to the type of unary predicates.
pub const PREALLOCATED_HANDLE_TYPE_UNARY_PREDICATE: Handle = 5;
/// A pre-allocated handle used to refer to the type of the polymorphic equality
/// symbol.
pub const PREALLOCATED_HANDLE_TYPE_EQUALITY: Handle = 6;
/// A pre-allocated handle used to refer to the type of binary predicates.
pub const PREALLOCATED_HANDLE_TYPE_BINARY_PREDICATE: Handle = 7;
/// A pre-allocated handle used to refer to the type of unary connectives.
pub const PREALLOCATED_HANDLE_TYPE_UNARY_CONNECTIVE: Handle = 8;
/// A pre-allocated handle used to refer to the type of binary connectives.
pub const PREALLOCATED_HANDLE_TYPE_BINARY_CONNECTIVE: Handle = 9;
/// A pre-allocated handle used to refer to the type of polymorphic quantifiers.
pub const PREALLOCATED_HANDLE_TYPE_QUANTIFIER: Handle = 10;
/// A pre-allocated handle used to refer to the truth constant.
pub const PREALLOCATED_HANDLE_CONSTANT_TRUE: Handle = 11;
/// A pre-allocated handle used to refer to the falsity constant.
pub const PREALLOCATED_HANDLE_CONSTANT_FALSE: Handle = 12;
/// A pre-allocated handle used to refer to the negation constant.
pub const PREALLOCATED_HANDLE_CONSTANT_NEGATION: Handle = 13;
/// A pre-allocated handle used to refer to the binary conjunction connective.
pub const PREALLOCATED_HANDLE_CONSTANT_CONJUNCTION: Handle = 14;
/// A pre-allocated handle used to refer to the binary disjunction connective.
pub const PREALLOCATED_HANDLE_CONSTANT_DISJUNCTION: Handle = 15;
/// A pre-allocated handle used to refer to the binary implication connective.
pub const PREALLOCATED_HANDLE_CONSTANT_IMPLICATION: Handle = 16;
/// A pre-allocated handle used to refer to the universal quantifier constant.
pub const PREALLOCATED_HANDLE_CONSTANT_FORALL: Handle = 17;
/// A pre-allocated handle used to refer to the existential quantifier constant.
pub const PREALLOCATED_HANDLE_CONSTANT_EXISTS: Handle = 18;
/// A pre-allocated handle used to refer to the truth term, the truth constant
/// lifted into a term.
pub const PREALLOCATED_HANDLE_TERM_TRUE: Handle = 19;
/// A pre-allocated handle used to refer to the falsity term, the falsity
/// constant lifted into a term.
pub const PREALLOCATED_HANDLE_TERM_FALSE: Handle = 20;
/// A pre-allocated handle used to refer to the negation term, the negation
/// constant lifted into a term.
pub const PREALLOCATED_HANDLE_TERM_NEGATION: Handle = 21;
/// A pre-allocated handle used to refer to the conjunction term, the
/// conjunction constant lifted into a term.
pub const PREALLOCATED_HANDLE_TERM_CONJUNCTION: Handle = 22;
/// A pre-allocated handle used to refer to the disjunction term, the
/// disjunction constant lifted into a term.
pub const PREALLOCATED_HANDLE_TERM_DISJUNCTION: Handle = 23;
/// A pre-allocated handle used to refer to the implication term, the
/// implication constant lifted into a term.
pub const PREALLOCATED_HANDLE_TERM_IMPLICATION: Handle = 24;
/// A pre-allocated handle used to refer to the equality term, the equality
/// constant lifted into a term.
pub const PREALLOCATED_HANDLE_TERM_EQUALITY: Handle = 25;
/// A pre-allocated handle used to refer to the universal quantifier term, the
/// universal quantifier constant lifted into a term.
pub const PREALLOCATED_HANDLE_TERM_FORALL: Handle = 26;
/// A pre-allocated handle used to refer to the existential quantifier term, the
/// existential quantifier constant lifted into a term.
pub const PREALLOCATED_HANDLE_TERM_EXISTS: Handle = 27;
