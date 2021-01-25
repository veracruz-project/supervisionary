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
    error_code::ErrorCode,
    handle::{
        issue_handle, Handle, PREALLOCATED_HANDLE_TYPE_ALPHA, PREALLOCATED_HANDLE_TYPE_BETA,
        PREALLOCATED_HANDLE_TYPE_BINARY_CONNECTIVE, PREALLOCATED_HANDLE_TYPE_BINARY_PREDICATE,
        PREALLOCATED_HANDLE_TYPE_EQUALITY, PREALLOCATED_HANDLE_TYPE_FORMER_ARROW,
        PREALLOCATED_HANDLE_TYPE_FORMER_PROP, PREALLOCATED_HANDLE_TYPE_PROP,
        PREALLOCATED_HANDLE_TYPE_QUANTIFIER, PREALLOCATED_HANDLE_TYPE_UNARY_CONNECTIVE,
        PREALLOCATED_HANDLE_TYPE_UNARY_PREDICATE,
    },
    type_former::{is_type_former_registered, type_former_arity},
};
use lazy_static::lazy_static;
use std::{collections::HashMap, convert::TryInto, sync::Mutex};

////////////////////////////////////////////////////////////////////////////////
// Miscellaneous material.
////////////////////////////////////////////////////////////////////////////////

/// The error message used when panicking if the lock on the type table cannot
/// be obtained.
const TABLE_LOCK_ERROR: &str = "Failed to obtain lock on type table.";

/// The error message used when panicking if a dangling handle is detected in a
/// type.
const DANGLING_HANDLE_ERROR: &str = "Kernel invariant failed: dangling handle.";

/// The error message used when panicking if a dangling handle is detected in a
/// type.
const PRIMITIVE_CONSTRUCTION_ERROR: &str =
    "Kernel invariant failed: failed to construct a kernel primitive.";

////////////////////////////////////////////////////////////////////////////////
// Types, proper.
////////////////////////////////////////////////////////////////////////////////

/// We use Strings to represent variable names.
pub type Name = String;

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
    Variable { name: Name },
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

    /// The alpha type-variable, "A".
    #[inline]
    pub fn alpha() -> Self {
        Type::Variable {
            name: String::from("A"),
        }
    }

    /// The beta type-variable, "B".
    #[inline]
    pub fn beta() -> Self {
        Type::Variable {
            name: String::from("B"),
        }
    }

    /// The primitive `Prop` type for HOL.
    #[inline]
    pub fn prop() -> Self {
        Type::combination(PREALLOCATED_HANDLE_TYPE_FORMER_PROP, Vec::new())
            .expect(PRIMITIVE_CONSTRUCTION_ERROR)
    }

    /// Constructs a function type from a domain and range type.
    #[inline]
    pub fn function(domain: Self, range: Self) -> Self {
        Type::combination(PREALLOCATED_HANDLE_TYPE_FORMER_ARROW, vec![domain, range])
            .expect(PRIMITIVE_CONSTRUCTION_ERROR)
    }

    /// The type of unary logical connectives, `Prop -> Prop`.
    #[inline]
    pub fn unary_connective() -> Self {
        Type::function(Type::prop(), Type::prop())
    }

    /// The type of binary logical connectives, `Prop -> (Prop -> Prop)`.
    #[inline]
    pub fn binary_connective() -> Self {
        Type::function(Type::prop(), Type::function(Type::prop(), Type::prop()))
    }

    /// The type of polymorphic unary predicates, `A -> Prop`.
    #[inline]
    pub fn polymorphic_unary_predicate() -> Self {
        Type::function(Type::alpha(), Type::prop())
    }

    /// The type of polymorphic binary predicates, `A -> (A -> Prop)`.
    #[inline]
    pub fn polymorphic_binary_predicate() -> Self {
        Type::function(Type::prop(), Type::function(Type::prop(), Type::prop()))
    }

    /// The type of polymorphic quantifiers, `(A -> Prop) -> Prop`.
    #[inline]
    pub fn polymorphic_quantifier() -> Self {
        Type::function(Type::function(Type::alpha(), Type::prop()), Type::prop())
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

        if handle == PREALLOCATED_HANDLE_TYPE_FORMER_ARROW && args.len() == 2 {
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
            handle == PREALLOCATED_HANDLE_TYPE_FORMER_PROP && args.is_empty()
        } else {
            false
        }
    }

    /// Returns `true` iff the type is well-formed, containing no dangling
    /// handles to any other types.
    ///
    /// Note that is all "registration" functions for terms, types, constants,
    /// and so on, preserve the invariant that they only ever allow a kernel
    /// object to be registered if it is well-formed, then this check only needs
    /// to be shallow: only the handles actually appearing at the very "top" of
    /// the type need to be checked to ensure that the entire type is
    /// well-formed.
    pub fn is_well_formed(&self) -> bool {
        match self {
            Type::Variable { .. } => true,
            Type::Combination { former, arguments } => {
                if !is_type_former_registered(former) {
                    return false;
                }

                arguments.iter().all(|a| is_type_registered(a))
            }
        }
    }
}

////////////////////////////////////////////////////////////////////////////////
// Interlude: the type table.
////////////////////////////////////////////////////////////////////////////////

lazy_static! {
    static ref TYPE_TABLE: Mutex<HashMap<Handle, Type>> = {
        let mut table = HashMap::new();

        table.insert(PREALLOCATED_HANDLE_TYPE_ALPHA, Type::alpha());
        table.insert(PREALLOCATED_HANDLE_TYPE_BETA, Type::beta());
        table.insert(PREALLOCATED_HANDLE_TYPE_PROP, Type::prop());
        table.insert(
            PREALLOCATED_HANDLE_TYPE_UNARY_CONNECTIVE,
            Type::function(Type::prop(), Type::prop()),
        );
        table.insert(
            PREALLOCATED_HANDLE_TYPE_BINARY_CONNECTIVE,
            Type::function(Type::prop(), Type::function(Type::prop(), Type::prop())),
        );
        table.insert(
            PREALLOCATED_HANDLE_TYPE_UNARY_PREDICATE,
            Type::function(Type::alpha(), Type::prop()),
        );
        table.insert(
            PREALLOCATED_HANDLE_TYPE_BINARY_PREDICATE,
            Type::function(Type::alpha(), Type::function(Type::alpha(), Type::prop())),
        );
        table.insert(
            PREALLOCATED_HANDLE_TYPE_QUANTIFIER,
            Type::function(Type::function(Type::alpha(), Type::prop()), Type::prop()),
        );

        Mutex::new(table)
    };
}

/// Registers a new type in the type table, returning the handle to the type if
/// it is already registered, otherwise inventing a new handle and returning
/// that.  Note that the complexity of registering a type in the table therefore
/// becomes linear in the number of types currently registered in the table, but
/// enforces maximal *sharing* in the sense that types are not registered
/// unnecessarily and every handle is therefore a unique reference to any type.
/// As a result, equality checking on type can be shallow, looking only at
/// handles without actually recursively fetching types from the table and
/// examining their structure.
///
/// Returns `Err(ErrorCode::TypeNotWellformed)` if the type `tau` is not
/// well-formed in the sense that it contains dangling handles.
///
/// Will **panic** if a lock on the type table cannot be obtained or if the type
/// `tau` is not well-formed, and includes dangling handles.
pub fn register_type(tau: Type) -> Result<Handle, ErrorCode> {
    let mut table = TYPE_TABLE.lock().expect(TABLE_LOCK_ERROR);

    if !tau.is_well_formed() {
        return Err(ErrorCode::TypeNotWellformed);
    }

    for (handle, registered) in table.iter() {
        if registered == &tau {
            return Ok(*handle);
        }
    }

    let fresh = issue_handle();

    table.insert(fresh, tau);

    Ok(fresh)
}

/// Returns `Some(tau)` iff a type `tau` associated with the handle is
/// registered in the type-table.
///
/// Will **panic** if a lock on the type-table cannot be obtained.
#[inline]
pub fn _type(handle: &Handle) -> Option<Type> {
    TYPE_TABLE
        .lock()
        .expect(TABLE_LOCK_ERROR)
        .get(handle)
        .map(|t| t.clone())
}

/// Returns `true` iff a type is associated with the handle is registered in the
/// type table.
///
/// Will **panic** if a lock on the type table cannot be obtained.
#[inline]
pub fn is_type_registered(handle: &Handle) -> bool {
    _type(handle).is_some()
}

////////////////////////////////////////////////////////////////////////////////
// Back to the material...
////////////////////////////////////////////////////////////////////////////////

impl Type {
    /// Creates a new type-former type from a handle to the underlying
    /// type-former and a list of handles to the arguments.
    ///
    /// Returns `Err(ErrorCode::NoSuchTypeFormerRegistered)` if `handle` does
    /// not point to any registered type-former in the type-former table.
    /// Returns `Err(ErrorCode::MismatchedArity)` if the length of `arguments`
    /// does not match the length of the registered arity for `handle`.  Returns
    /// `Err(ErrorCode::NoSuchTypeRegistered)` if any of the handles in
    /// `arguments` is not registered in the type-table.
    pub fn combination(former: Handle, arguments: Vec<Handle>) -> Result<Self, ErrorCode> {
        let arity = type_former_arity(&handle).ok_or(ErrorCode::NoSuchTypeFormerRegistered)?;

        if arity != arguments.len() {
            return Err(ErrorCode::MismatchedArity);
        }

        if arguments.iter().any(|a| !is_type_registered(a)) {
            return Err(ErrorCode::NoSuchTypeRegistered);
        }

        Ok(Type::Combination { former, arguments })
    }

    /// Returns the set of type variables appearing in the type.  Duplicate
    /// variables are removed, with each variable appearing at most once.
    pub fn ftv(self) -> Vec<Name> {
        let mut buffer = Vec::new();
        let mut work = vec![self];

        while let Some(tau) = work.pop() {
            match tau {
                Type::Variable { name } => buffer.push(name),
                Type::Combination { mut arguments, .. } => work.append(&mut arguments),
            }
        }

        buffer.sort();
        buffer.dedup();

        buffer
    }

    /// Performs a type-substitution on the current type, replacing all
    /// occurrences of the name `domain` for the range type, `range`.  Note that
    /// this can allocate new types in the type-table as a side-effect of
    /// substitution.
    pub fn substitute<T>(&self, domain: T, range: Self) -> Self
    where
        T: Into<Name> + Clone,
    {
        match self {
            Type::Variable { name } => {
                if name == domain.into() {
                    range
                } else {
                    self.clone()
                }
            }
            Type::Combination {
                former,
                mut arguments,
            } => {
                let arguments = arguments
                    .iter_mut()
                    .map(|handle| {
                        *handle = register_type(
                            _type(handle)
                                .expect(DANGLING_HANDLE_ERROR)
                                .substitute(domain.clone(), range.clone()),
                        )
                        .expect(DANGLING_HANDLE_ERROR)
                    })
                    .collect();

                Type::Combination {
                    former: *former,
                    arguments,
                }
            }
        }
    }
}

////////////////////////////////////////////////////////////////////////////////
// Trait implementations.
////////////////////////////////////////////////////////////////////////////////

/// Partial projection from type-variables into (a reference to) names.
impl<'a> TryInto<&'a Name> for Type {
    type Error = ();

    #[inline]
    fn try_into(self) -> Result<&'a Name, Self::Error> {
        self.split_variable().ok_or(Err(()))
    }
}

/// Partial projection from type-variables into names.
impl TryInto<Name> for Type {
    type Error = ();

    #[inline]
    fn try_into(self) -> Result<Name, Self::Error> {
        self.split_variable().map(|v| *v).ok_or(Err(()))
    }
}

/// Partial projection from combinations into pairs of (references to)
/// type-former handles and lists of argument handles.
impl<'a> TryInto<(&'a Handle, &'a Vec<Handle>)> for Type {
    type Error = ();

    #[inline]
    fn try_into(self) -> Result<(&'a Handle, &'a Vec<Handle>), Self::Error> {
        self.split_combination().ok_or(Err(()))
    }
}

/// Partial projection from combinations into pairs of type-former handles and
/// lists of argument handles.
impl TryInto<(Handle, Vec<Handle>)> for Type {
    type Error = ();

    #[inline]
    fn try_into(self) -> Result<(Handle, Vec<Handle>), Self::Error> {
        self.split_combination()
            .map(|(h, args)| (*h, args.iter().cloned().collect()))
            .ok_or(Err(()))
    }
}

/// Injection from names into type-variables.
impl From<Name> for Type {
    #[inline]
    fn from(name: Name) -> Self {
        Type::variable(name)
    }
}

/// Injection from (references to) names into type-variables.
impl From<&Name> for Type {
    #[inline]
    fn from(name: &Name) -> Self {
        Type::variable(name)
    }
}
