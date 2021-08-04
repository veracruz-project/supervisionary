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

use crate::handle::{tags, Handle};
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
