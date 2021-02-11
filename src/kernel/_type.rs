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

use crate::kernel::{
    handle::{
        Handle, PREALLOCATED_HANDLE_TYPE_ALPHA, PREALLOCATED_HANDLE_TYPE_FORMER_ARROW,
        PREALLOCATED_HANDLE_TYPE_FORMER_PROP, PREALLOCATED_HANDLE_TYPE_PROP,
        PREALLOCATED_HANDLE_TYPE_UNARY_CONNECTIVE, PREALLOCATED_HANDLE_TYPE_UNARY_PREDICATE,
    },
    name::Name,
};

use lazy_static::lazy_static;

////////////////////////////////////////////////////////////////////////////////
// Types, proper.
////////////////////////////////////////////////////////////////////////////////

/// Types are either *variables* with a name, which can be substituted for other
/// types, or *type-formers* which are fully applied to a list of arguments
/// matching the declared arity of the type-former in the kernel's type-former
/// table.  Type-formers are used to make more complex types (for example, the
/// function type, the `Option` type, the `List` type, the product type, and so
/// on) from other types, and are also used to introduce type-constants, which
/// correspond to type-formers applied to a zero-length argument list, (for
/// example, the natural numbers, the primitive `Prop` type, the real numbers,
/// and so on).
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Type {
    /// A type-variable with a name.  We use `String` to represent names.
    Variable {
        /// The name of the variable.
        name: Name,
    },
    /// A type-former, consisting of a reference to a previously-declared
    /// type-former, in the kernel's type-former table, and a list of type
    /// arguments whose length must always match the declared arity of the
    /// type-former.
    Combination {
        /// The reference to a previously-declared type-former in the kernel's
        /// type-former table.
        former: Handle,
        /// The arguments to the type-former: a list of references to
        /// previously-defined types in the kernel's type table.
        arguments: Vec<Handle>,
    },
}

impl Type {
    /// Creates a new type-variable type from a given name.
    #[inline]
    pub fn variable<T>(name: T) -> Self
    where
        T: Into<Name>,
    {
        Type::Variable { name: name.into() }
    }

    /// Creates a new type combination, applying a type-former to a collection
    /// of arguments.  Note that no checking for well-formedness is made, here.
    #[inline]
    pub fn combination<T>(former: Handle, arguments: T) -> Self
    where
        T: Iterator<Item = Handle>,
    {
        Type::Combination {
            former,
            arguments: arguments.collect(),
        }
    }

    /// Constructs a function type from a domain and range type.
    #[inline]
    pub fn function(domain: Handle, range: Handle) -> Self {
        Type::combination(
            PREALLOCATED_HANDLE_TYPE_FORMER_ARROW,
            vec![domain, range].into_iter(),
        )
    }

    /// Returns `Some(name)` iff the type is a type-variable with name, `name`.
    pub fn split_variable(&self) -> Option<&String> {
        if let Type::Variable { name } = self {
            Some(name)
        } else {
            None
        }
    }

    /// Returns `Some((former, arguments))` iff the type is a fully-applied
    /// type-former, with handle `handle`, applied to a list of arguments,
    /// `arguments`.
    pub fn split_combination(&self) -> Option<(&Handle, &Vec<Handle>)> {
        if let Type::Combination { former, arguments } = self {
            Some((former, arguments))
        } else {
            None
        }
    }

    /// Returns `Some((dom, rng))` iff the type is a function type between
    /// domain type `dom` and range type `rng`.
    pub fn split_function(&self) -> Option<(&Handle, &Handle)> {
        let (handle, args) = self.split_combination()?;

        if handle == &PREALLOCATED_HANDLE_TYPE_FORMER_ARROW && args.len() == 2 {
            Some((&args[0], &args[1]))
        } else {
            None
        }
    }

    /// Returns `true` iff the type is a type-variable.
    #[inline]
    pub fn is_variable(&self) -> bool {
        self.split_variable().is_some()
    }

    /// Returns `true` iff the type is a fully-applied type-former.
    #[inline]
    pub fn is_combination(&self) -> bool {
        self.split_combination().is_some()
    }

    /// Returns `true` iff the type is a function type.
    #[inline]
    pub fn is_function(&self) -> bool {
        self.split_function().is_some()
    }

    /// Returns `true` iff the type is a proposition type.
    pub fn is_prop(&self) -> bool {
        if let Some((handle, args)) = self.split_combination() {
            handle == &PREALLOCATED_HANDLE_TYPE_FORMER_PROP && args.is_empty()
        } else {
            false
        }
    }
}

////////////////////////////////////////////////////////////////////////////////
// Predefined types.
////////////////////////////////////////////////////////////////////////////////

lazy_static! {
    /// The "alpha" type-variable, `A`.
    pub static ref TYPE_ALPHA: Type = Type::Variable {
        name: String::from("A"),
    };

    /// The "beta" type-variable, `B`.
    pub static ref TYPE_BETA: Type = Type::Variable {
        name: String::from("B"),
    };

    /// The type of propositions, `Prop`.
    pub static ref TYPE_PROP: Type = Type::Combination {
        former: PREALLOCATED_HANDLE_TYPE_FORMER_PROP,
        arguments: Vec::new(),
    };

    /// The type of unary logical connectives, `Prop -> Prop`.
    pub static ref TYPE_UNARY_CONNECTIVE: Type = Type::Combination {
        former: PREALLOCATED_HANDLE_TYPE_FORMER_ARROW,
        arguments: vec![PREALLOCATED_HANDLE_TYPE_PROP, PREALLOCATED_HANDLE_TYPE_PROP],
    };

    /// The type of binary logical connectives, `Prop -> (Prop -> Prop)`.
    pub static ref TYPE_BINARY_CONNECTIVE: Type = Type::Combination {
        former: PREALLOCATED_HANDLE_TYPE_FORMER_ARROW,
        arguments: vec![
            PREALLOCATED_HANDLE_TYPE_PROP,
            PREALLOCATED_HANDLE_TYPE_UNARY_CONNECTIVE,
        ],
    };

    /// The type of polymorphic unary predicates, `A -> Prop`.
    pub static ref TYPE_POLYMORPHIC_UNARY_PREDICATE: Type = Type::Combination {
        former: PREALLOCATED_HANDLE_TYPE_FORMER_ARROW,
        arguments: vec![
            PREALLOCATED_HANDLE_TYPE_ALPHA,
            PREALLOCATED_HANDLE_TYPE_PROP,
        ],
    };

    /// The type of polymorphic binary predicates, `A -> (A -> Prop)`.
    pub static ref TYPE_POLYMORPHIC_BINARY_PREDICATE: Type = Type::Combination {
        former: PREALLOCATED_HANDLE_TYPE_FORMER_ARROW,
        arguments: vec![
            PREALLOCATED_HANDLE_TYPE_ALPHA,
            PREALLOCATED_HANDLE_TYPE_UNARY_PREDICATE,
        ],
    };

    /// The type of polymorphic quantifiers, `(A -> Prop) -> Prop`.
    pub static ref TYPE_POLYMORPHIC_QUANTIFIER: Type = Type::Combination {
        former: PREALLOCATED_HANDLE_TYPE_FORMER_ARROW,
        arguments: vec![
            PREALLOCATED_HANDLE_TYPE_UNARY_PREDICATE,
            PREALLOCATED_HANDLE_TYPE_PROP,
        ],
    };
}
