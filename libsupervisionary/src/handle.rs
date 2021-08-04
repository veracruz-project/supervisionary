//! # Kernel handles
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

impl<T> Handle<T>
where
    T: tags::IsTag,
{
    /// Creates a new kernel handle from a raw handle and some phantom data
    /// constraining the handle to be of a particular tag-type.
    #[inline]
    pub(crate) const fn new(handle: usize, marker: PhantomData<T>) -> Self {
        Self { handle, marker }
    }
}

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
