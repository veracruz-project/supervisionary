//! # Kernel object handles
//!
//! Kernel objects are manipulated only by the kernel, so prover-space code
//! needs some way of naming the object that should be manipulated by the
//! kernel.  In Supervisionary, we use *handles* for this purpose, which are
//! simply machine words suitable for passing across the kernel/prover-space ABI
//! boundary.
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

use std::{
    fmt,
    fmt::{Display, Formatter},
    marker::PhantomData,
    ops::Deref,
};

////////////////////////////////////////////////////////////////////////////////
// Handle tags.
////////////////////////////////////////////////////////////////////////////////

pub mod tags {
    #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub struct TypeFormer;

    #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub struct Type;

    #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub struct Constant;

    #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub struct Term;

    #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub struct Theorem;

    pub trait IsTag {}

    impl IsTag for TypeFormer {}

    impl IsTag for Type {}

    impl IsTag for Constant {}

    impl IsTag for Term {}

    impl IsTag for Theorem {}
}

////////////////////////////////////////////////////////////////////////////////
// Tagged handles.
////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Handle<T>
where
    T: tags::IsTag,
{
    /// We use the Rust `usize` type as our handle type.  Note that on modern 64-bit
    /// systems this is implemented as a 64-bit unsigned integer.
    handle: usize,
    /// The phantom data binding the tag type, `T`.
    marker: PhantomData<T>,
}

/// The upper-bound (exclusive) of the preallocated handles.
pub const PREALLOCATED_HANDLE_UPPER_BOUND: usize = 28;

/// Returns `true` iff the handle is a pre-allocated handle built into the
/// kernel.
#[inline]
pub fn is_preallocated<T>(handle: Handle<T>) -> bool
where
    T: tags::IsTag,
{
    *handle < PREALLOCATED_HANDLE_UPPER_BOUND
}

////////////////////////////////////////////////////////////////////////////////
// Pre-allocated handles for kernel objects.
////////////////////////////////////////////////////////////////////////////////

/// A pre-allocated handle used to refer to the `Prop` type-former.
pub const PREALLOCATED_HANDLE_TYPE_FORMER_PROP: Handle<tags::TypeFormer> = Handle {
    handle: 0,
    marker: PhantomData,
};
/// A pre-allocated handle used to refer to the function-space type-former.
pub const PREALLOCATED_HANDLE_TYPE_FORMER_ARROW: Handle<tags::TypeFormer> = Handle {
    handle: 1,
    marker: PhantomData,
};
/// A pre-allocated handle used to refer to the type-variable `A`.
pub const PREALLOCATED_HANDLE_TYPE_ALPHA: Handle<tags::Type> = Handle {
    handle: 2,
    marker: PhantomData,
};
/// A pre-allocated handle used to refer to the type-variable `B`.
pub const PREALLOCATED_HANDLE_TYPE_BETA: Handle<tags::Type> = Handle {
    handle: 3,
    marker: PhantomData,
};
/// A pre-allocated handle used to refer to the `Prop` type.
pub const PREALLOCATED_HANDLE_TYPE_PROP: Handle<tags::Type> = Handle {
    handle: 4,
    marker: PhantomData,
};
/// A pre-allocated handle used to refer to the type of unary predicates.
pub const PREALLOCATED_HANDLE_TYPE_UNARY_PREDICATE: Handle<tags::Type> = Handle {
    handle: 5,
    marker: PhantomData,
};
/// A pre-allocated handle used to refer to the type of binary predicates.
pub const PREALLOCATED_HANDLE_TYPE_BINARY_PREDICATE: Handle<tags::Type> = Handle {
    handle: 6,
    marker: PhantomData,
};
/// A pre-allocated handle used to refer to the type of unary connectives.
pub const PREALLOCATED_HANDLE_TYPE_UNARY_CONNECTIVE: Handle<tags::Type> = Handle {
    handle: 7,
    marker: PhantomData,
};
/// A pre-allocated handle used to refer to the type of binary connectives.
pub const PREALLOCATED_HANDLE_TYPE_BINARY_CONNECTIVE: Handle<tags::Type> = Handle {
    handle: 8,
    marker: PhantomData,
};
/// A pre-allocated handle used to refer to the type of polymorphic quantifiers.
pub const PREALLOCATED_HANDLE_TYPE_QUANTIFIER: Handle<tags::Type> = Handle {
    handle: 9,
    marker: PhantomData,
};
/// A pre-allocated handle used to refer to the truth constant.
pub const PREALLOCATED_HANDLE_CONSTANT_TRUE: Handle<tags::Constant> = Handle {
    handle: 10,
    marker: PhantomData,
};
/// A pre-allocated handle used to refer to the falsity constant.
pub const PREALLOCATED_HANDLE_CONSTANT_FALSE: Handle<tags::Constant> = Handle {
    handle: 11,
    marker: PhantomData,
};
/// A pre-allocated handle used to refer to the negation constant.
pub const PREALLOCATED_HANDLE_CONSTANT_NEGATION: Handle<tags::Constant> = Handle {
    handle: 12,
    marker: PhantomData,
};
/// A pre-allocated handle used to refer to the binary conjunction connective.
pub const PREALLOCATED_HANDLE_CONSTANT_CONJUNCTION: Handle<tags::Constant> = Handle {
    handle: 13,
    marker: PhantomData,
};
/// A pre-allocated handle used to refer to the binary disjunction connective.
pub const PREALLOCATED_HANDLE_CONSTANT_DISJUNCTION: Handle<tags::Constant> = Handle {
    handle: 14,
    marker: PhantomData,
};
/// A pre-allocated handle used to refer to the binary implication connective.
pub const PREALLOCATED_HANDLE_CONSTANT_IMPLICATION: Handle<tags::Constant> = Handle {
    handle: 15,
    marker: PhantomData,
};
/// A pre-allocated handle used to refer to the universal quantifier constant.
pub const PREALLOCATED_HANDLE_CONSTANT_FORALL: Handle<tags::Constant> = Handle {
    handle: 16,
    marker: PhantomData,
};
/// A pre-allocated handle used to refer to the existential quantifier constant.
pub const PREALLOCATED_HANDLE_CONSTANT_EXISTS: Handle<tags::Constant> = Handle {
    handle: 17,
    marker: PhantomData,
};
/// A pre-allocated handle used to refer to the equality constant.
pub const PREALLOCATED_HANDLE_CONSTANT_EQUALITY: Handle<tags::Constant> = Handle {
    handle: 18,
    marker: PhantomData,
};
/// A pre-allocated handle used to refer to the truth term, the truth constant
/// lifted into a term.
pub const PREALLOCATED_HANDLE_TERM_TRUE: Handle<tags::Term> = Handle {
    handle: 19,
    marker: PhantomData,
};
/// A pre-allocated handle used to refer to the falsity term, the falsity
/// constant lifted into a term.
pub const PREALLOCATED_HANDLE_TERM_FALSE: Handle<tags::Term> = Handle {
    handle: 20,
    marker: PhantomData,
};
/// A pre-allocated handle used to refer to the negation term, the negation
/// constant lifted into a term.
pub const PREALLOCATED_HANDLE_TERM_NEGATION: Handle<tags::Term> = Handle {
    handle: 21,
    marker: PhantomData,
};
/// A pre-allocated handle used to refer to the conjunction term, the
/// conjunction constant lifted into a term.
pub const PREALLOCATED_HANDLE_TERM_CONJUNCTION: Handle<tags::Term> = Handle {
    handle: 22,
    marker: PhantomData,
};
/// A pre-allocated handle used to refer to the disjunction term, the
/// disjunction constant lifted into a term.
pub const PREALLOCATED_HANDLE_TERM_DISJUNCTION: Handle<tags::Term> = Handle {
    handle: 23,
    marker: PhantomData,
};
/// A pre-allocated handle used to refer to the implication term, the
/// implication constant lifted into a term.
pub const PREALLOCATED_HANDLE_TERM_IMPLICATION: Handle<tags::Term> = Handle {
    handle: 24,
    marker: PhantomData,
};
/// A pre-allocated handle used to refer to the equality term, the equality
/// constant lifted into a term.
pub const PREALLOCATED_HANDLE_TERM_EQUALITY: Handle<tags::Term> = Handle {
    handle: 25,
    marker: PhantomData,
};
/// A pre-allocated handle used to refer to the universal quantifier term, the
/// universal quantifier constant lifted into a term.
pub const PREALLOCATED_HANDLE_TERM_FORALL: Handle<tags::Term> = Handle {
    handle: 26,
    marker: PhantomData,
};
/// A pre-allocated handle used to refer to the existential quantifier term, the
/// existential quantifier constant lifted into a term.
pub const PREALLOCATED_HANDLE_TERM_EXISTS: Handle<tags::Term> = Handle {
    handle: 27,
    marker: PhantomData,
};

////////////////////////////////////////////////////////////////////////////////
// Trait implementations.
////////////////////////////////////////////////////////////////////////////////

impl<T> Deref for Handle<T>
where
    T: tags::IsTag,
{
    type Target = usize;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.handle
    }
}

impl<T> From<usize> for Handle<T>
where
    T: tags::IsTag,
{
    #[inline]
    fn from(handle: usize) -> Self {
        Handle {
            handle,
            marker: PhantomData,
        }
    }
}

impl Display for Handle<tags::Term> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{} (term handle)", self.handle)
    }
}

impl Display for Handle<tags::Constant> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{} (constant handle)", self.handle)
    }
}

impl Display for Handle<tags::TypeFormer> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{} (type-former handle)", self.handle)
    }
}

impl Display for Handle<tags::Type> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{} (type handle)", self.handle)
    }
}
