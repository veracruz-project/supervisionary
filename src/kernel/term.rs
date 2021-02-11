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

use crate::wasmi::{
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
    /// Note that the kernel should ensure that the `handle` handle should not
    /// dangle, and if `_type` is `Some(h)` for some handle `h` then `h` should
    /// not dangle, either.
    Constant {
        /// A handle to the declared constant.
        handle: Handle,
        /// Either `None`, indicating that the constant is being used at its
        /// declared type, or `Some(h)` indicating that the constant is being
        /// used at a specialized type, where `h` is the corresponding handle to
        /// this type.
        _type: Option<Handle>,
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
    pub fn constant(handle: Handle, _type: Option<Handle>) -> Self {
        Term::Constant {
            handle: constant,
            _type,
        }
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
    pub fn split_constant(&self) -> Option<(&Handle, &Option<Handle>)> {
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
    _type: None,
};

pub const TERM_FALSE_CONSTANT: Term = Term::Constant {
    handle: PREALLOCATED_HANDLE_CONSTANT_FALSE,
    _type: None,
};

/*
    #[inline]
    pub fn _true() -> Self {

    }

    #[inline]
    pub fn _false() -> Self {
        Term::Constant {
            handle: PREALLOCATED_HANDLE_CONSTANT_FALSE,
            _type: None,
        }
    }

    pub fn conjunction() -> Self {
        Term::Constant {
            handle: PREALLOCATED_HANDLE_CONSTANT_CONJUNCTION,
            _type: None,
        }
    }

    pub fn disjunction() -> Self {
        Term::Constant {
            handle: PREALLOCATED_HANDLE_CONSTANT_DISJUNCTION,
            _type: None,
        }
    }

    pub fn implication() -> Self {
        Term::Constant {
            handle: PREALLOCATED_HANDLE_CONSTANT_IMPLICATION,
            _type: None,
        }
    }

    pub fn equality() -> Self {
        Term::Constant {
            handle: PREALLOCATED_HANDLE_CONSTANT_EQUALITY,
            _type: None,
        }
    }

    pub fn negation() -> Self {
        Term::Constant {
            handle: PREALLOCATED_HANDLE_CONSTANT_NEGATION,
            _type: None,
        }
    }

    pub fn forall() -> Self {
        Term
    }
}
*/
