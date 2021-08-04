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

use crate::handle::{tags, Handle};
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
