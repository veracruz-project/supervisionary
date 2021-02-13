//! # HOL types
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

use crate::kernel::handle::{
    PREALLOCATED_HANDLE_TERM_CONJUNCTION, PREALLOCATED_HANDLE_TERM_DISJUNCTION,
    PREALLOCATED_HANDLE_TERM_EQUALITY, PREALLOCATED_HANDLE_TERM_EXISTS,
    PREALLOCATED_HANDLE_TERM_FORALL, PREALLOCATED_HANDLE_TERM_IMPLICATION,
    PREALLOCATED_HANDLE_TERM_NEGATION, PREALLOCATED_HANDLE_TYPE_BINARY_CONNECTIVE,
    PREALLOCATED_HANDLE_TYPE_BINARY_PREDICATE, PREALLOCATED_HANDLE_TYPE_PROP,
    PREALLOCATED_HANDLE_TYPE_QUANTIFIER, PREALLOCATED_HANDLE_TYPE_UNARY_CONNECTIVE,
};
use crate::kernel::{
    handle::{
        Handle, PREALLOCATED_HANDLE_CONSTANT_CONJUNCTION, PREALLOCATED_HANDLE_CONSTANT_DISJUNCTION,
        PREALLOCATED_HANDLE_CONSTANT_EQUALITY, PREALLOCATED_HANDLE_CONSTANT_FALSE,
        PREALLOCATED_HANDLE_CONSTANT_IMPLICATION, PREALLOCATED_HANDLE_CONSTANT_NEGATION,
        PREALLOCATED_HANDLE_CONSTANT_TRUE,
    },
    name::Name,
};

////////////////////////////////////////////////////////////////////////////////
// Terms, proper.
////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Debug, Hash)]
pub enum Term {
    /// Variables of the simply-typed lambda-calculus.  All variables are
    /// explicitly typed.  Not that two variables are equal when their names and
    /// types are both equal.
    ///
    /// Note that the kernel should ensure that the `_type` handle does not
    /// dangle.
    Variable {
        /// The name of the variable.
        name: Name,
        /// A handle to the explicit type of the variable.
        _type: Handle,
    },
    /// Constants.  Constants can either be used at their declared type, in the
    /// constant-table, in which case we set the `_type` field to be `None` to
    /// represent this, or can be specialized to a substitutive instance of the
    /// declared type, in which case we set the `_type` field to be `Some(h)`
    /// where `h` is the handle for the specialized type.  Note that two
    /// constants are equal when their names are equal and their types are
    /// equal.
    ///
    /// Note that the kernel should ensure that the `handle` and `_type` handles
    /// should not dangle.
    Constant {
        /// A handle to the declared constant.
        handle: Handle,
        /// A handle pointing-to the type of the constant.
        _type: Handle,
    },
    /// An application of one type, `left`, to another, `right`.  Left must be
    /// of functional type for this term to be type-correct.
    ///
    /// Note that the kernel must ensure that neither the `left` nor `right`
    /// handles dangle.
    Application {
        /// A handle to the functional term being applied to the argument,
        /// `right`.
        left: Handle,
        /// A handle to the argument term being consumed by the functional term,
        /// `left`.
        right: Handle,
    },
    /// A lambda-abstraction, introducing a new function with argument `name`
    /// of type `_type`, with body `body`.
    ///
    /// Note that the kernel must ensure that neither `_type` nor `body` handles
    /// dangle.
    Lambda {
        /// The name of the newly-introduced function's formal parameter.
        name: Name,
        /// A handle to the type of the function's formal parameter.
        _type: Handle,
        /// A handle to the lambda-term representing the function's body.
        body: Handle,
    },
}

impl Term {
    #[inline]
    pub fn variable<T>(name: T, handle: Handle) -> Self
    where
        T: Into<Name>,
    {
        Term::Variable {
            name: name.into(),
            _type: handle,
        }
    }

    #[inline]
    pub fn constant(handle: Handle, _type: Handle) -> Self {
        Term::Constant { handle, _type }
    }

    #[inline]
    pub fn application(left: Handle, right: Handle) -> Self {
        Term::Application { left, right }
    }

    #[inline]
    pub fn lambda<T>(name: T, _type: Handle, body: Handle) -> Self
    where
        T: Into<Name>,
    {
        Term::Lambda {
            name: name.into(),
            _type,
            body,
        }
    }

    /// Returns `Some((name, handle))` iff the term is a variable with name,
    /// `name`, and a handle pointing to its type, `handle`.
    pub fn split_variable(&self) -> Option<(&Name, &Handle)> {
        if let Term::Variable { name, _type } = self {
            Some((name, _type))
        } else {
            None
        }
    }

    /// Returns `Some((handle, opt))` iff the term is a constant with a handle
    /// pointing to a registered constant in the constant-table, and an optional
    /// handle pointing to a type, `opt`.  If `opt` is `None` then the constant
    /// has the type registered in the constant-table under the handle,
    /// `handle`.
    pub fn split_constant(&self) -> Option<(&Handle, &Handle)> {
        if let Term::Constant { handle, _type } = self {
            Some((handle, _type))
        } else {
            None
        }
    }

    /// Returns `Some((left, right))` iff the term is an application of one term
    /// to another.
    pub fn split_application(&self) -> Option<(&Handle, &Handle)> {
        if let Term::Application { left, right } = self {
            Some((left, right))
        } else {
            None
        }
    }

    /// Returns `Some((name, type, body))` iff the term is a lambda-abstraction
    /// with bound name, `name`, handle to a type, `type`, and handle to a body
    /// expression, `body`.
    pub fn split_lambda(&self) -> Option<(&Name, &Handle, &Handle)> {
        if let Term::Lambda { name, _type, body } = self {
            Some((name, _type, body))
        } else {
            None
        }
    }

    /// Returns `true` iff the term is a variable.
    #[inline]
    pub fn is_variable(&self) -> bool {
        self.split_variable().is_some()
    }

    /// Returns `true` iff the term is a constant.
    #[inline]
    pub fn is_constant(&self) -> bool {
        self.split_constant().is_some()
    }

    /// Returns `true` iff the term is an application of one term to another.
    #[inline]
    pub fn is_application(&self) -> bool {
        self.split_application().is_some()
    }

    /// Returns `true` iff the term is a lambda-abstraction.
    #[inline]
    pub fn is_lambda(&self) -> bool {
        self.split_lambda().is_some()
    }
}

////////////////////////////////////////////////////////////////////////////////
// Useful constants.
////////////////////////////////////////////////////////////////////////////////

pub const TERM_TRUE_CONSTANT: Term = Term::Constant {
    handle: PREALLOCATED_HANDLE_CONSTANT_TRUE,
    _type: PREALLOCATED_HANDLE_TYPE_PROP,
};

pub const TERM_FALSE_CONSTANT: Term = Term::Constant {
    handle: PREALLOCATED_HANDLE_CONSTANT_FALSE,
    _type: PREALLOCATED_HANDLE_TYPE_PROP,
};

pub const TERM_NEGATION_CONSTANT: Term = Term::Constant {
    handle: PREALLOCATED_HANDLE_TERM_NEGATION,
    _type: PREALLOCATED_HANDLE_TYPE_UNARY_CONNECTIVE,
};

pub const TERM_CONJUNCTION_CONSTANT: Term = Term::Constant {
    handle: PREALLOCATED_HANDLE_TERM_CONJUNCTION,
    _type: PREALLOCATED_HANDLE_TYPE_BINARY_CONNECTIVE,
};

pub const TERM_DISJUNCTION_CONSTANT: Term = Term::Constant {
    handle: PREALLOCATED_HANDLE_TERM_DISJUNCTION,
    _type: PREALLOCATED_HANDLE_TYPE_BINARY_CONNECTIVE,
};

pub const TERM_IMPLICATION_CONSTANT: Term = Term::Constant {
    handle: PREALLOCATED_HANDLE_TERM_IMPLICATION,
    _type: PREALLOCATED_HANDLE_TYPE_BINARY_CONNECTIVE,
};

pub const TERM_FORALL_CONSTANT: Term = Term::Constant {
    handle: PREALLOCATED_HANDLE_TERM_FORALL,
    _type: PREALLOCATED_HANDLE_TYPE_QUANTIFIER,
};

pub const TERM_EXISTS_CONSTANT: Term = Term::Constant {
    handle: PREALLOCATED_HANDLE_TERM_EXISTS,
    _type: PREALLOCATED_HANDLE_TYPE_QUANTIFIER,
};

pub const TERM_EQUALITY_CONSTANT: Term = Term::Constant {
    handle: PREALLOCATED_HANDLE_TERM_EQUALITY,
    _type: PREALLOCATED_HANDLE_TYPE_BINARY_PREDICATE,
};
