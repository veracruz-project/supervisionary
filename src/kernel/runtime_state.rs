//! # The runtime state
//!
//! *Note that this is trusted code.*
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

use crate::kernel::kernel_panic::kernel_error;
use crate::kernel::{
    _type::{
        Type, TYPE_ALPHA, TYPE_BETA, TYPE_BINARY_CONNECTIVE, TYPE_POLYMORPHIC_BINARY_PREDICATE,
        TYPE_POLYMORPHIC_QUANTIFIER, TYPE_POLYMORPHIC_UNARY_PREDICATE, TYPE_PROP,
        TYPE_UNARY_CONNECTIVE,
    },
    error_code::ErrorCode,
    handle::{
        tags, Handle, PREALLOCATED_HANDLE_CONSTANT_CONJUNCTION,
        PREALLOCATED_HANDLE_CONSTANT_DISJUNCTION, PREALLOCATED_HANDLE_CONSTANT_EQUALITY,
        PREALLOCATED_HANDLE_CONSTANT_EXISTS, PREALLOCATED_HANDLE_CONSTANT_FALSE,
        PREALLOCATED_HANDLE_CONSTANT_FORALL, PREALLOCATED_HANDLE_CONSTANT_IMPLICATION,
        PREALLOCATED_HANDLE_CONSTANT_NEGATION, PREALLOCATED_HANDLE_CONSTANT_TRUE,
        PREALLOCATED_HANDLE_TERM_CONJUNCTION, PREALLOCATED_HANDLE_TERM_DISJUNCTION,
        PREALLOCATED_HANDLE_TERM_EQUALITY, PREALLOCATED_HANDLE_TERM_EXISTS,
        PREALLOCATED_HANDLE_TERM_FALSE, PREALLOCATED_HANDLE_TERM_FORALL,
        PREALLOCATED_HANDLE_TERM_IMPLICATION, PREALLOCATED_HANDLE_TERM_NEGATION,
        PREALLOCATED_HANDLE_TERM_TRUE, PREALLOCATED_HANDLE_TYPE_ALPHA,
        PREALLOCATED_HANDLE_TYPE_BETA, PREALLOCATED_HANDLE_TYPE_BINARY_CONNECTIVE,
        PREALLOCATED_HANDLE_TYPE_BINARY_PREDICATE, PREALLOCATED_HANDLE_TYPE_FORMER_ARROW,
        PREALLOCATED_HANDLE_TYPE_FORMER_PROP, PREALLOCATED_HANDLE_TYPE_PROP,
        PREALLOCATED_HANDLE_TYPE_QUANTIFIER, PREALLOCATED_HANDLE_TYPE_UNARY_CONNECTIVE,
        PREALLOCATED_HANDLE_TYPE_UNARY_PREDICATE, PREALLOCATED_HANDLE_UPPER_BOUND,
    },
    kernel_panic::{
        kernel_info, kernel_panic, DANGLING_HANDLE_ERROR, HANDLE_EXHAUST_ERROR,
        PRIMITIVE_CONSTRUCTION_ERROR,
    },
    name::Name,
    term::{
        Term, TERM_CONJUNCTION_CONSTANT, TERM_DISJUNCTION_CONSTANT, TERM_EQUALITY_CONSTANT,
        TERM_EXISTS_CONSTANT, TERM_FALSE_CONSTANT, TERM_FORALL_CONSTANT, TERM_IMPLICATION_CONSTANT,
        TERM_NEGATION_CONSTANT, TERM_TRUE_CONSTANT,
    },
    theorem::Theorem,
};
use std::fmt::Debug;
use std::{borrow::Borrow, collections::HashMap, fmt::Display, iter::FromIterator};

////////////////////////////////////////////////////////////////////////////////
// The runtime state.
////////////////////////////////////////////////////////////////////////////////

/// The runtime state of the kernel, containing the various tables of kernel
/// objects, indexed by handles.  The WASM host interface manipulates this
/// state.
#[derive(Clone, Debug)]
pub struct RuntimeState {
    /// The next handle to issue by the runtime state when a new kernel object
    /// is registered.
    next_handle: usize,
    /// The table of registered type-formers.  Handles are essentially names for
    /// type-formers.
    type_formers: HashMap<Handle<tags::TypeFormer>, usize>,
    /// The table of types.  The kernel enforces maximal sharing, wherein any
    /// attempt to register a previously-registered type means that the handle
    /// pointing to the registered type is returned.
    types: HashMap<Handle<tags::Type>, Type>,
    /// The table of constants, associating handles for constants to handles for
    /// types.  Handles are essentially names for constants.
    constants: HashMap<Handle<tags::Constant>, Handle<tags::Type>>,
    /// The table of terms.  The kernel enforces maximal sharing, wherein any
    /// attempt to register a previously-registered term (up-to
    /// alpha-equivalence) means that the handle pointing to the registered term
    /// is returned.
    terms: HashMap<Handle<tags::Term>, Term>,
    /// The table of theorems.  The kernel enforces maximal sharing, wherein any
    /// attempt to register a previously-registered theorem (up-to
    /// alpha-equivalence of the conclusion and hypotheses) means that the
    /// handle pointing to the registered theorem is returned.
    theorems: HashMap<Handle<tags::Theorem>, Theorem>,
}

impl RuntimeState {
    #[inline]
    pub fn new() -> Self {
        Default::default()
    }

    /// Issues a fresh handle.  Callers should not rely on this returning
    /// consecutive handles.
    ///
    /// Will **panic** if issued handles are exhausted.
    fn issue_handle<T>(&mut self) -> Handle<T>
    where
        T: tags::IsTag,
    {
        let next = self.next_handle;

        match self.next_handle.checked_add(1) {
            None => kernel_panic(HANDLE_EXHAUST_ERROR),
            Some(next) => self.next_handle = next,
        }

        kernel_info(format!("Generating fresh handle: {}.", next));

        return Handle::from(next);
    }

    ////////////////////////////////////////////////////////////////////////////
    // Type-former related material.
    ////////////////////////////////////////////////////////////////////////////

    /// Registers a new type-former with a declared arity with the runtime
    /// state.  Returns the handle to the newly-registered type-former.
    pub fn type_former_register<T>(&mut self, arity: T) -> Handle<tags::TypeFormer>
    where
        T: Into<usize> + Clone,
    {
        kernel_info(format!(
            "Registering new type-former with arity: {}.",
            arity.clone().into()
        ));
        let handle = self.issue_handle();
        self.type_formers.insert(handle.clone(), arity.into());
        handle
    }

    /// Returns Some(`arity`) if the type-former pointed-to by `handle` has
    /// arity `arity`.
    #[inline]
    pub fn type_former_resolve<T>(&self, handle: T) -> Option<&usize>
    where
        T: Borrow<Handle<tags::TypeFormer>>,
    {
        kernel_info(format!(
            "Resolving type-former with handle: {}.",
            handle.borrow()
        ));
        self.type_formers.get(handle.borrow())
    }

    /// Returns `true` iff `handle` points to a type-former registered with the
    /// runtime state.
    #[inline]
    pub fn type_former_is_registered<T>(&self, handle: T) -> bool
    where
        T: Borrow<Handle<tags::TypeFormer>>,
    {
        kernel_info(format!(
            "Checking type-former {} is registered.",
            handle.borrow()
        ));

        self.type_former_resolve(handle).is_some()
    }

    ////////////////////////////////////////////////////////////////////////////
    // Type related material.
    ////////////////////////////////////////////////////////////////////////////

    /// Admits a type into the runtime state's type table.  If the type is
    /// already registered (up-to syntactic equality) in the type-table, then
    /// the existing handle is returned, to enforce sharing.
    ///
    /// Functions calling this should ensure that the type argument, `tau`, is
    /// well-formed before calling.
    fn admit_type(&mut self, tau: Type) -> Handle<tags::Type> {
        for (handle, registered) in self.types.iter() {
            if registered == &tau {
                kernel_info(format!("Type already registered with handle: {}.", handle));
                return handle.clone();
            }
        }

        let handle = self.issue_handle();
        self.types.insert(handle.clone(), tau);

        kernel_info(format!("Type newly registered with handle: {}.", handle));

        handle
    }

    /// Returns `Some(tau)` iff the handle points to a type, `tau` in the
    /// runtime state's type-table.
    #[inline]
    pub fn resolve_type_handle<T>(&self, handle: T) -> Option<&Type>
    where
        T: Borrow<Handle<tags::Type>>,
    {
        kernel_info(format!("Resolving type with handle: {}.", handle.borrow()));

        self.types.get(handle.borrow())
    }

    /// Returns `true` iff the handle points to a type, `tau`, in the runtime
    /// state's type-table.
    #[inline]
    pub fn type_is_registered<T>(&self, handle: T) -> bool
    where
        T: Borrow<Handle<tags::Type>>,
    {
        kernel_info(format!("Checking type {} is registered.", handle.borrow()));

        self.resolve_type_handle(handle).is_some()
    }

    /// Registers a new type in the runtime state's type-table with a given
    /// name.  Returns the handle of the newly-allocated type (or the existing
    /// handle, if the type-variable already appears in the type-table).
    #[inline]
    pub fn type_register_variable<T>(&mut self, name: T) -> Handle<tags::Type>
    where
        T: Into<Name> + Clone,
    {
        kernel_info(format!(
            "Registering type-variable: {}.",
            name.clone().into()
        ));

        self.admit_type(Type::variable(name))
    }

    /// Registers a new type combination, consisting of a type-former applied to
    /// a list of type arguments, in the runtime state's type-table.  Returns
    /// `Ok(handle)`, where `handle` is the handle of the newly-allocated type,
    /// if registration is successful.
    ///
    /// # Errors
    ///
    /// Returns `Err(ErrorCode::NoSuchTypeFormerRegistered)` if `former` does
    /// not point-to any type-former in the runtime state's type-former table.
    ///
    /// Returns `Err(ErrorCode::NoSuchTypeRegistered)` if any handle appearing
    /// in `arguments` does not point-to any type in the runtime state's
    /// type-table.
    ///
    /// Returns `Err(ErrorCode::MismatchedArity)` if the length of `arguments`
    /// does not match the registered arity of `former` in the runtime state's
    /// type-former table.
    pub fn type_register_combination<T, U>(
        &mut self,
        former: T,
        arguments: Vec<U>,
    ) -> Result<Handle<tags::Type>, ErrorCode>
    where
        T: Into<Handle<tags::TypeFormer>> + Clone + Display,
        U: Into<Handle<tags::Type>> + Clone + Debug,
    {
        kernel_info(format!(
            "Registering type-former {} applied to arguments {:?}.",
            former.clone(),
            arguments.clone()
        ));

        let former = former.into();

        let arity = self
            .type_former_resolve(former.clone())
            .ok_or({
                kernel_error("Type-former handle is not registered.")
                ErrorCode::NoSuchTypeFormerRegistered
            })?;

        if !arguments
            .iter()
            .all(|a| self.type_is_registered(a.clone().into()))
        {
            kernel_error("Not all type argument handles are registered.");

            return Err(ErrorCode::NoSuchTypeRegistered);
        }

        if arguments.len() != *arity {
            kernel_error(format!(
                "Number of arguments provided does not match registered arity: {}.",
                arity,
            ));

            return Err(ErrorCode::MismatchedArity);
        }

        Ok(self.admit_type(Type::combination(former, arguments)))
    }

    /// Registers a new function type out of a domain and range type in the
    /// runtime state's type-table.  Returns `Ok(handle)`, where `handle` is the
    /// handle of the newly-allocated type, if registration is successful.
    ///
    /// **Note**: this is merely a convenience function for client code, and
    /// can be implemented in terms of `register_type_combination()`.
    ///
    /// # Errors
    ///
    /// Returns `Err(ErrorCode::NoSuchTypeRegistered)` if either `domain` or
    /// `range` do not point-to a type in the runtime state's type-table.
    pub fn type_register_function<T>(
        &mut self,
        domain: T,
        range: T,
    ) -> Result<Handle<tags::Type>, ErrorCode>
    where
        T: Into<Handle<tags::Type>> + Display,
    {
        kernel_info(format!("Registering function type with domain {} and range {}.", domain, range));

        let domain = domain.into();
        let range = range.into();

        if !self.type_is_registered(&domain) {
            kernel_error("Domain type handle is not registered.");

            return Err(ErrorCode::NoSuchTypeRegistered);
        }

        if !self.type_is_registered(&range) {
            kernel_error("Range type handle is not registered.");

            return Err(ErrorCode::NoSuchTypeRegistered);
        }

        Ok(self.admit_type(Type::Combination {
            former: PREALLOCATED_HANDLE_TYPE_FORMER_ARROW,
            arguments: vec![domain, range],
        }))
    }

    /// Returns `Ok(name)` iff the type pointed-to by `handle` in the runtime
    /// state's type-table is a type-variable with name `name`.
    ///
    /// # Errors
    ///
    /// Returns `Err(ErrorCode::NoSuchTypeRegistered)` if `handle` does not
    /// point-to a type in the runtime state's type-table.
    ///
    /// Returns `Err(ErrorCode::NotATypeVariable)` if the type pointed-to by
    /// `handle` in the runtime state's type-table is not a type-variable.
    pub fn type_split_variable<T>(&self, handle: T) -> Result<&Name, ErrorCode>
    where
        T: Borrow<Handle<tags::Type>>,
    {
        kernel_info(format!("Splitting handle {} into type variable.", handle.borrow()));

        if let Some(tau) = self.resolve_type_handle(handle) {
            tau.split_variable().ok_or({
                kernel_error("Handle is not a type variable.");

                ErrorCode::NotATypeVariable
            })
        } else {
            kernel_error("Unknown handle.");

            Err(ErrorCode::NoSuchTypeRegistered)
        }
    }

    /// Returns `Ok((former, args))` iff the type pointed-to by `handle` in the
    /// runtime state's type-table is a combination type consisting of a
    /// type-former with handle `former` applied to a list of type arguments,
    /// `args`.
    ///
    /// # Errors
    ///
    /// Returns `Err(ErrorCode::NoSuchTypeRegistered)` if `handle` does not
    /// point-to a type in the runtime state's type-table.
    ///
    /// Returns `Err(ErrorCode::NotATypeCombination)` if the type pointed-to by
    /// `handle` in the runtime state's type-table is not a combination type.
    pub fn type_split_combination<T>(
        &self,
        handle: T,
    ) -> Result<(&Handle<tags::TypeFormer>, &Vec<Handle<tags::Type>>), ErrorCode>
    where
        T: Borrow<Handle<tags::Type>>,
    {
        kernel_info(format!("Splitting handle {} into type combination.", handle));

        if let Some(tau) = self.resolve_type_handle(handle) {
            tau.split_combination()
                .ok_or(ErrorCode::NotATypeCombination)
        } else {
            Err(ErrorCode::NoSuchTypeRegistered)
        }
    }

    /// Returns `Ok((domain, range))` iff the type pointed-to by `handle` in the
    /// runtime state's type-table is a function type from `domain` to `range`.
    ///
    /// # Errors
    ///
    /// Returns `Err(ErrorCode::NoSuchTypeRegistered)` if `handle` does not
    /// point-to a type in the runtime state's type-table.
    ///
    /// Returns `Err(ErrorCode::NotAFunctionType)` if the type pointed-to by
    /// `handle` in the runtime state's type-table is not a function type.
    pub fn type_split_function<T>(
        &self,
        handle: T,
    ) -> Result<(&Handle<tags::Type>, &Handle<tags::Type>), ErrorCode>
    where
        T: Borrow<Handle<tags::Type>>,
    {
        if let Some(tau) = self.resolve_type_handle(handle) {
            tau.split_function().ok_or(ErrorCode::NotAFunctionType)
        } else {
            Err(ErrorCode::NoSuchTypeRegistered)
        }
    }

    /// Returns `Ok(true)` iff the type pointed-to by `handle` in the runtime
    /// state's type-table is a type-variable.
    ///
    /// # Errors
    ///
    /// Returns `Err(ErrorCode::NoSuchTypeRegistered)` if `handle` does not
    /// point-to a type in the runtime state's type-table.
    #[inline]
    pub fn type_test_is_variable<T>(&self, handle: T) -> Result<bool, ErrorCode>
    where
        T: Borrow<Handle<tags::Type>>,
    {
        Ok(self
            .resolve_type_handle(handle)
            .ok_or(ErrorCode::NoSuchTypeRegistered)?
            .split_variable()
            .is_some())
    }

    /// Returns `Ok(true)` iff the type pointed-to by `handle` in the runtime
    /// state's type-table is a combination type.
    ///
    /// # Errors
    ///
    /// Returns `Err(ErrorCode::NoSuchTypeRegistered)` if `handle` does not
    /// point-to a type in the runtime state's type-table.
    #[inline]
    pub fn type_test_is_combination<T>(&self, handle: T) -> Result<bool, ErrorCode>
    where
        T: Borrow<Handle<tags::Type>>,
    {
        Ok(self
            .resolve_type_handle(handle)
            .ok_or(ErrorCode::NoSuchTypeRegistered)?
            .split_combination()
            .is_some())
    }

    /// Returns `Ok(true)` iff the type pointed-to by `handle` in the runtime
    /// state's type-table is a function type.
    ///
    /// # Errors
    ///
    /// Returns `Err(ErrorCode::NoSuchTypeRegistered)` if `handle` does not
    /// point-to a type in the runtime state's type-table.
    #[inline]
    pub fn type_test_is_function<T>(&self, handle: T) -> Result<bool, ErrorCode>
    where
        T: Borrow<Handle<tags::Type>>,
    {
        Ok(self
            .resolve_type_handle(handle)
            .ok_or(ErrorCode::NoSuchTypeRegistered)?
            .split_function()
            .is_some())
    }

    /// Returns `Ok(vs)` where `vs` is the set of variables appearing in the
    /// type pointed-to by `handle` in the runtime state's type-table.
    ///
    /// # Errors
    ///
    /// Returns `Err(ErrorCode::NoSuchTypeRegistered)` if `handle` does not
    /// point-to a type in the runtime state's type-table.
    ///
    /// Will raise a kernel panic if the type pointed-to by `handle` is
    /// malformed.
    pub fn type_ftv<T>(&self, handle: T) -> Result<Vec<&Name>, ErrorCode>
    where
        T: Borrow<Handle<tags::Type>>,
    {
        let tau = self
            .resolve_type_handle(handle)
            .ok_or(ErrorCode::NoSuchTypeRegistered)?;

        let mut ftv = Vec::new();
        let mut work_list = vec![tau];

        while let Some(tau) = work_list.pop() {
            match tau {
                Type::Variable { name } => ftv.push(name),
                Type::Combination { arguments, .. } => {
                    let mut arguments = arguments
                        .iter()
                        .map(|a| self.resolve_type_handle(a).expect(DANGLING_HANDLE_ERROR))
                        .collect();
                    work_list.append(&mut arguments);
                }
            }
        }

        ftv.sort();
        ftv.dedup();

        Ok(ftv)
    }

    /// Instantiates a type pointed-to by the handle `tau`, using the type
    /// substitution `sigma`.
    ///
    /// # Errors
    ///
    /// Returns `Err(ErrorCode::NoSuchTypeRegistered)` if `tau` does not point
    /// to a type in the runtime state's type-table.
    ///
    /// Returns `Err(ErrorCode::NoSuchTypeRegistered)` if any of the handles
    /// contained in `sigma` do not point to a type in the runtime state's
    /// type-table.
    ///
    /// Will raise a kernel panic if any of the manipulated types are malformed.
    pub fn type_substitute<T, U, V>(
        &mut self,
        tau: T,
        sigma: Vec<(U, V)>,
    ) -> Result<Handle<tags::Type>, ErrorCode>
    where
        T: Borrow<Handle<tags::Type>>,
        U: Into<Name> + Clone,
        V: Into<Handle<tags::Type>> + Clone,
    {
        let mut tau = self
            .resolve_type_handle(tau)
            .ok_or(ErrorCode::NoSuchTypeRegistered)?
            .clone();

        for (domain, range) in sigma.clone() {
            let range = self
                .resolve_type_handle(&range.into())
                .ok_or(ErrorCode::NoSuchTypeRegistered)?;

            match tau {
                Type::Variable { ref name } => {
                    if name == &domain.into() {
                        tau = range.clone();
                    }
                }
                Type::Combination { former, arguments } => {
                    let mut args = vec![];

                    for a in arguments.iter() {
                        let argument = self.type_substitute(a, sigma.clone())?;
                        args.push(argument);
                    }

                    tau = Type::Combination {
                        former,
                        arguments: args,
                    }
                }
            }
        }

        Ok(self.admit_type(tau))
    }

    ////////////////////////////////////////////////////////////////////////////
    // Constant related material.
    ////////////////////////////////////////////////////////////////////////////

    /// Registers a new constant, with a type pointed-to by `handle`, in the
    /// runtime state's constant-table.  Generates a fresh handle to name the
    /// constant.
    ///
    /// # Errors
    ///
    /// Returns `Err(ErrorCode::NoSuchTypeRegistered)` if `handle` does not
    /// point to a registered type in the runtime state's type-table.
    pub fn constant_register<T>(&mut self, handle: T) -> Result<Handle<tags::Constant>, ErrorCode>
    where
        T: Into<Handle<tags::Type>> + Clone,
    {
        if !self.type_is_registered(handle.clone().into()) {
            return Err(ErrorCode::NoSuchTypeRegistered);
        }

        let fresh = self.issue_handle();
        self.constants.insert(fresh.clone(), handle.into());
        Ok(fresh)
    }

    /// Returns `Ok(tau)` iff `handle` points to a registered constant, with
    /// type handle `tau`, in the runtime state's type-table.
    ///
    /// # Errors
    ///
    /// Returns `Err(ErrorCode::NoSuchConstantRegistered)` if `handle` does not
    /// point-to any constant in the runtime state's constant-table.
    #[inline]
    pub fn constant_resolve<T>(&self, handle: T) -> Result<&Handle<tags::Type>, ErrorCode>
    where
        T: Borrow<Handle<tags::Constant>>,
    {
        self.constants
            .get(handle.borrow())
            .ok_or(ErrorCode::NoSuchConstantRegistered)
    }

    /// Returns `true` iff `handle` points-to a registered constant in the
    /// runtime state's constant table.
    #[inline]
    pub fn constant_is_registered<T>(&self, handle: T) -> bool
    where
        T: Borrow<Handle<tags::Constant>>,
    {
        self.constant_resolve(handle).is_ok()
    }

    ////////////////////////////////////////////////////////////////////////////
    // Term related material.
    ////////////////////////////////////////////////////////////////////////////

    /// Admits a new term into the runtime state's term-table.  If any term
    /// exists in the runtime state's term-table that is alpha-equivalent to
    /// `trm` then the handle for that existing term is returned.  Otherwise, a
    /// fresh handle is generated and the term `trm` is admitted.  It is
    /// expected that `trm` has been checked for well-formedness before this
    /// function is called.
    fn admit_term(&mut self, trm: Term) -> Handle<tags::Term> {
        for (handle, registered) in self.terms.clone().iter() {
            if self
                .is_alpha_equivalent_inner(&trm, &registered)
                .expect(DANGLING_HANDLE_ERROR)
            {
                return handle.clone();
            }
        }

        let fresh = self.issue_handle();
        self.terms.insert(fresh.clone(), trm);
        fresh
    }

    /// Registers a new term variable, with name `name` and with the type
    /// pointed-to by handle in the runtime state's type-table.
    ///
    /// # Errors
    ///
    /// Returns `Err(ErrorCode::NoSuchTypeRegistered)` if `handle` does not
    /// point to any type in the runtime state's type-table.
    pub fn term_register_variable<T, U>(
        &mut self,
        name: T,
        handle: U,
    ) -> Result<Handle<tags::Term>, ErrorCode>
    where
        T: Into<Name>,
        U: Into<Handle<tags::Type>> + Clone,
    {
        if !self.type_is_registered(handle.clone().into()) {
            return Err(ErrorCode::NoSuchTypeRegistered);
        }

        Ok(self.admit_term(Term::variable(name, handle)))
    }

    /// Registers a new term constant, lifting the handle pointing-to a
    /// registered constant in the runtime state's constant-table into a term.
    /// Applies the type-substitution `sigma` to the constant's registered type
    /// and uses the resulting type as the type of the lifted constant.
    ///
    /// # Errors
    ///
    /// Returns `Err(ErrorCode::NoSuchConstantRegistered)` if `handle` does not
    /// point-to a registered constant in the runtime state's constant-table.
    ///
    /// Returns `Err(ErrorCode::NoSuchTypeRegistered)` if any handle appearing
    /// in `sigma` does not point-to a registered type in the runtime state's
    /// type-table.
    pub fn term_register_constant<T, U, V>(
        &mut self,
        handle: T,
        type_substitution: Vec<(U, V)>,
    ) -> Result<Handle<tags::Term>, ErrorCode>
    where
        T: Into<Handle<tags::Constant>> + Clone,
        U: Into<Name> + Clone,
        V: Into<Handle<tags::Type>> + Clone,
    {
        let cnst = self.constant_resolve(handle.clone().into())?.clone();

        let tau = self.type_substitute(cnst, type_substitution)?;

        Ok(self.admit_term(Term::constant(handle, tau)))
    }

    /// Registers a new application of the term pointed-to by `left` to the term
    /// pointed-to by `right`.  Performs a type-check of the application,
    /// failing if it will not result in a typeable term.
    ///
    /// # Errors
    ///
    /// Returns `Err(ErrorCode::NoSuchTermRegistered)` if `left` or `right` do
    /// not pointed to registered terms in the runtime state's term-table.
    ///
    /// Returns `Err(ErrorCode::NotAFunctionType)` if the type of the term
    /// pointed-to by `left` does not have a function type.
    ///
    /// Returns `Err(ErrorCode::DomainTypeMismatch)` if `left`, `right`, or the
    /// application of `left` to `right` are not typeable.
    pub fn term_register_application<T, U>(
        &mut self,
        left: T,
        right: U,
    ) -> Result<Handle<tags::Term>, ErrorCode>
    where
        T: Into<Handle<tags::Term>> + Clone,
        U: Into<Handle<tags::Term>> + Clone,
    {
        if !self.is_term_registered(left.clone().into()) {
            return Err(ErrorCode::NoSuchTermRegistered);
        }

        if !self.is_term_registered(right.clone().into()) {
            return Err(ErrorCode::NoSuchTermRegistered);
        }

        let ltau = self.term_type_infer(left.clone().into())?;
        let rtau = self.term_type_infer(right.clone().into())?;

        let (dom, _rng) = self.type_split_function(&ltau)?;

        if dom != &rtau {
            return Err(ErrorCode::DomainTypeMismatch);
        }

        Ok(self.admit_term(Term::application(left, right)))
    }

    /// Registers a new lambda-abstraction into the runtime state's term-table
    /// with a name, a type pointed-to by the handle `tau`, and a body term
    /// pointed-to by the handle `handle`.
    ///
    /// # Errors
    ///
    /// Returns `Err(ErrorCode::NoSuchTypeRegistered)` if `tau` does not
    /// point-to a registered type in the runtime state's type-table.
    ///
    /// Returns `Err(ErrorCode::NoSuchTermRegistered)`  if `handle` does not
    /// point-to a registered term in the runtime state's term-table.
    pub fn term_register_lambda<T, U, V>(
        &mut self,
        name: T,
        tau: U,
        body: V,
    ) -> Result<Handle<tags::Term>, ErrorCode>
    where
        T: Into<Name>,
        U: Into<Handle<tags::Type>> + Clone,
        V: Into<Handle<tags::Term>> + Clone,
    {
        if !self.type_is_registered(tau.clone().into()) {
            return Err(ErrorCode::NoSuchTypeRegistered);
        }

        if !self.is_term_registered(body.clone().into()) {
            return Err(ErrorCode::NoSuchTermRegistered);
        }

        Ok(self.admit_term(Term::lambda(name, tau, body)))
    }

    /// Registers a new negation of the term pointed-to by `term` in the runtime
    /// state's term-table.
    ///
    /// # Errors
    ///
    /// Returns `Err(ErrorCode::NoSuchTermRegistered)` if `term` does not
    /// point-to any registered term in the runtime state's term-table.
    ///
    /// Returns `Err(ErrorCode::NotAProposition)` if the term pointed-to by
    /// `term` is not a proposition.
    pub fn term_register_negation<T>(&mut self, term: T) -> Result<Handle<tags::Term>, ErrorCode>
    where
        T: Into<Handle<tags::Term>> + Clone,
    {
        if !self.term_type_is_proposition(term.clone().into())? {
            return Err(ErrorCode::NotAProposition);
        }

        self.term_register_application(PREALLOCATED_HANDLE_TERM_NEGATION, term)
    }

    /// Registers a new equality between the terms pointed-to by `left` and
    /// `right`.  Correctly instantiates the polymorphic equality constant as
    /// part of term construction.
    ///
    /// # Errors
    ///
    /// Returns `Err(ErrorCode::NoSuchTermRegistered)` if either `left` or
    /// `right` don't point-to any registered term in the runtime state's
    /// term-table.
    ///
    /// Returns `Err(ErrorCode::DomainTypeMismatch)` if the types of the terms
    /// pointed-to by `left` and `right` are not equal.
    pub fn term_register_equality<T, U>(
        &mut self,
        left: T,
        right: U,
    ) -> Result<Handle<tags::Term>, ErrorCode>
    where
        T: Into<Handle<tags::Term>> + Clone,
        U: Into<Handle<tags::Term>> + Clone,
    {
        let ltau = self.term_type_infer(left.clone().into())?;
        let rtau = self.term_type_infer(right.clone().into())?;

        if ltau != rtau {
            return Err(ErrorCode::DomainTypeMismatch);
        }

        let spec = self.term_type_substitution(
            PREALLOCATED_HANDLE_TERM_EQUALITY,
            vec![(String::from("A"), ltau)],
        )?;

        let inner = self.term_register_application(spec, left)?;

        self.term_register_application(inner, right)
    }

    /// Registers a new disjunction between the terms pointed-to by `left` and
    /// `right`.
    ///
    /// # Errors
    ///
    /// Returns `Err(ErrorCode::NoSuchTermRegistered)` if either `left` or
    /// `right` don't point-to any registered term in the runtime state's
    /// term-table.
    ///
    /// Returns `Err(ErrorCode::NotAProposition)` if either of the terms
    /// pointed-to by `left` or `right` are not propositions.
    pub fn term_register_disjunction<T, U>(
        &mut self,
        left: T,
        right: U,
    ) -> Result<Handle<tags::Term>, ErrorCode>
    where
        T: Into<Handle<tags::Term>> + Clone,
        U: Into<Handle<tags::Term>> + Clone,
    {
        if !self.term_type_is_proposition(left.clone().into())? {
            return Err(ErrorCode::NotAProposition);
        }

        if !self.term_type_is_proposition(right.clone().into())? {
            return Err(ErrorCode::NotAProposition);
        }

        let inner = self.term_register_application(PREALLOCATED_HANDLE_TERM_DISJUNCTION, left)?;

        self.term_register_application(inner, right)
    }

    /// Registers a new conjunction between the terms pointed-to by `left` and
    /// `right`.
    ///
    /// # Errors
    ///
    /// Returns `Err(ErrorCode::NoSuchTermRegistered)` if either `left` or
    /// `right` don't point-to any registered term in the runtime state's
    /// term-table.
    ///
    /// Returns `Err(ErrorCode::NotAProposition)` if either of the terms
    /// pointed-to by `left` or `right` are not propositions.
    pub fn term_register_conjunction<T, U>(
        &mut self,
        left: T,
        right: U,
    ) -> Result<Handle<tags::Term>, ErrorCode>
    where
        T: Into<Handle<tags::Term>> + Clone,
        U: Into<Handle<tags::Term>> + Clone,
    {
        if !self.term_type_is_proposition(left.clone().into())? {
            return Err(ErrorCode::NotAProposition);
        }

        if !self.term_type_is_proposition(right.clone().into())? {
            return Err(ErrorCode::NotAProposition);
        }

        let inner = self.term_register_application(PREALLOCATED_HANDLE_TERM_CONJUNCTION, left)?;

        self.term_register_application(inner, right)
    }

    /// Registers a new implication between the terms pointed-to by `left` and
    /// `right`.
    ///
    /// # Errors
    ///
    /// Returns `Err(ErrorCode::NoSuchTermRegistered)` if either `left` or
    /// `right` don't point-to any registered term in the runtime state's
    /// term-table.
    ///
    /// Returns `Err(ErrorCode::NotAProposition)` if either of the terms
    /// pointed-to by `left` or `right` are not propositions.
    pub fn term_register_implication<T, U>(
        &mut self,
        left: T,
        right: U,
    ) -> Result<Handle<tags::Term>, ErrorCode>
    where
        T: Into<Handle<tags::Term>> + Clone,
        U: Into<Handle<tags::Term>> + Clone,
    {
        if !self.term_type_is_proposition(left.clone().into())? {
            return Err(ErrorCode::NotAProposition);
        }

        if !self.term_type_is_proposition(right.clone().into())? {
            return Err(ErrorCode::NotAProposition);
        }

        let inner = self.term_register_application(PREALLOCATED_HANDLE_TERM_IMPLICATION, left)?;

        self.term_register_application(inner, right)
    }

    /// Registers a new universal quantifier from a type, pointed-to by `tau`,
    /// a body, pointed to by `body`, and a name.
    ///
    /// # Errors
    ///
    /// Returns `Err(ErrorCode::NoSuchTermRegistered)` if `body` does not
    /// point-to any registered term in the runtime state's term-table.
    ///
    /// Returns `Err(ErrorCode::NotAProposition)` if the term pointed-to by
    /// `body` is not a proposition.
    ///
    /// Returns `Err(ErrorCode::NoSuchTypeRegistered)` if `tau` does not
    /// point-to any registered type in the runtime state's type-table.
    pub fn term_register_forall<T, U, V>(
        &mut self,
        name: T,
        tau: U,
        body: V,
    ) -> Result<Handle<tags::Term>, ErrorCode>
    where
        T: Into<Name>,
        U: Into<Handle<tags::Type>> + Clone,
        V: Into<Handle<tags::Term>> + Clone,
    {
        if !self.type_is_registered(tau.clone().into()) {
            return Err(ErrorCode::NoSuchTypeRegistered);
        }

        if !self.term_type_is_proposition(body.clone().into())? {
            return Err(ErrorCode::NotAProposition);
        }

        let lambda = self
            .term_register_lambda(name, tau.clone(), body)
            .expect(DANGLING_HANDLE_ERROR);

        let univ = self
            .term_type_substitution(
                PREALLOCATED_HANDLE_TERM_FORALL,
                vec![(String::from("A"), tau)],
            )
            .expect(DANGLING_HANDLE_ERROR);

        self.term_register_application(univ, lambda)
    }

    /// Registers a new existential quantifier from a type, pointed-to by
    /// `tau`, a body, pointed to by `body`, and a name.
    ///
    /// # Errors
    ///
    /// Returns `Err(ErrorCode::NoSuchTermRegistered)` if `body` does not
    /// point-to any registered term in the runtime state's term-table.
    ///
    /// Returns `Err(ErrorCode::NotAProposition)` if the term pointed-to by
    /// `body` is not a proposition.
    ///
    /// Returns `Err(ErrorCode::NoSuchTypeRegistered)` if `tau` does not
    /// point-to any registered type in the runtime state's type-table.
    pub fn term_register_exists<T, U, V>(
        &mut self,
        name: T,
        tau: U,
        body: V,
    ) -> Result<Handle<tags::Term>, ErrorCode>
    where
        T: Into<Name>,
        U: Into<Handle<tags::Type>> + Clone,
        V: Into<Handle<tags::Term>> + Clone,
    {
        if !self.type_is_registered(tau.clone().into()) {
            return Err(ErrorCode::NoSuchTypeRegistered);
        }

        if !self.term_type_is_proposition(body.clone().into())? {
            return Err(ErrorCode::NotAProposition);
        }

        let lambda = self
            .term_register_lambda(name, tau.clone(), body)
            .expect(DANGLING_HANDLE_ERROR);

        let univ = self
            .term_type_substitution(
                PREALLOCATED_HANDLE_TERM_EXISTS,
                vec![(String::from("A"), tau)],
            )
            .expect(DANGLING_HANDLE_ERROR);

        self.term_register_application(univ, lambda)
    }

    /// Returns `Ok(trm)` iff `handle` points-to the term `trm` in the runtime
    /// state's term-table.
    ///
    /// # Errors
    ///
    /// Returns `Err(ErrorCode::NoSuchTermRegistered)` if `handle` does not
    /// point-to any registered term in the runtime state's term-table.
    #[inline]
    pub fn resolve_term_handle<T>(&self, handle: T) -> Result<&Term, ErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        self.terms
            .get(handle.borrow())
            .ok_or(ErrorCode::NoSuchTermRegistered)
    }

    /// Returns `true` iff `handle` points-to a registered term in the runtime
    /// state's term-table.
    #[inline]
    pub fn is_term_registered<T>(&self, handle: T) -> bool
    where
        T: Borrow<Handle<tags::Term>>,
    {
        self.resolve_term_handle(handle).is_ok()
    }

    /// Returns `Some((name, _type))` if `handle` points-to a variable in the
    /// runtime state's term-table with name, `name`, and a handle pointing-to a
    /// type, `_type`.
    ///
    /// # Errors
    ///
    /// Returns `Err(ErrorCode::NoSuchTermRegistered)` if `handle` does not
    /// point-to any term in the runtime state's term-table.
    ///
    /// Returns `Err(ErrorCode::NotAVariable)` if the term pointed-to by
    /// `handle` is not a variable.
    pub fn term_split_variable<T>(
        &self,
        handle: T,
    ) -> Result<(&Name, &Handle<tags::Type>), ErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        let trm = self.resolve_term_handle(handle)?;

        if let Term::Variable { name, tau: _type } = trm {
            Ok((name, _type))
        } else {
            Err(ErrorCode::NotAVariable)
        }
    }

    /// Returns `Some((constant, _type))` if `handle` points-to a constant in
    /// the runtime state's term-table with a handle pointing to a constant,
    /// `constant`, and a handle pointing-to a type, `_type`.
    ///
    /// # Errors
    ///
    /// Returns `Err(ErrorCode::NoSuchTermRegistered)` if `handle` does not
    /// point-to any term in the runtime state's term-table.
    ///
    /// Returns `Err(ErrorCode::NotAConstant)` if the term pointed-to by
    /// `handle` is not a constant.
    pub fn term_split_constant<T>(
        &self,
        handle: T,
    ) -> Result<(&Handle<tags::Constant>, &Handle<tags::Type>), ErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        let trm = self.resolve_term_handle(handle)?;

        if let Term::Constant {
            constant: handle,
            tau: _type,
        } = trm
        {
            Ok((handle, _type))
        } else {
            Err(ErrorCode::NotAConstant)
        }
    }

    pub fn term_split_application<T>(
        &self,
        handle: T,
    ) -> Result<(&Handle<tags::Term>, &Handle<tags::Term>), ErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        let trm = self.resolve_term_handle(handle)?;

        if let Term::Application { left, right } = trm {
            Ok((left, right))
        } else {
            Err(ErrorCode::NotAnApplication)
        }
    }

    pub fn term_split_lambda<T>(
        &self,
        handle: T,
    ) -> Result<(&Name, &Handle<tags::Type>, &Handle<tags::Term>), ErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        let trm = self.resolve_term_handle(handle)?;

        if let Term::Lambda {
            name,
            tau: _type,
            body,
        } = trm
        {
            Ok((name, _type, body))
        } else {
            Err(ErrorCode::NotALambda)
        }
    }

    pub fn term_split_negation<T>(&self, handle: T) -> Result<&Handle<tags::Term>, ErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        let (left, right) = self
            .resolve_term_handle(handle)?
            .split_application()
            .ok_or(ErrorCode::NotANegation)?;

        let (constant, _tau) = self
            .term_split_constant(left)
            .map_err(|_e| ErrorCode::NotANegation)?;

        if constant != &PREALLOCATED_HANDLE_CONSTANT_NEGATION {
            return Err(ErrorCode::NotANegation);
        }

        Ok(right)
    }

    pub fn term_split_equality<T>(
        &self,
        handle: T,
    ) -> Result<(&Handle<tags::Term>, &Handle<tags::Term>), ErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        let (left, right) = self
            .resolve_term_handle(handle)?
            .split_application()
            .ok_or(ErrorCode::NotAnEquality)?;

        let (left, mid) = self
            .resolve_term_handle(left)
            .expect(DANGLING_HANDLE_ERROR)
            .split_application()
            .ok_or(ErrorCode::NotAnEquality)?;

        let (constant, _tau) = self
            .term_split_constant(left)
            .map_err(|_e| ErrorCode::NotAnEquality)?;

        if constant == &PREALLOCATED_HANDLE_CONSTANT_EQUALITY {
            Ok((mid, right))
        } else {
            Err(ErrorCode::NotAnEquality)
        }
    }

    pub fn term_split_disjunction<T>(
        &self,
        handle: T,
    ) -> Result<(&Handle<tags::Term>, &Handle<tags::Term>), ErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        let (left, right) = self
            .resolve_term_handle(handle)?
            .split_application()
            .ok_or(ErrorCode::NotADisjunction)?;

        let (left, mid) = self
            .resolve_term_handle(left)
            .expect(DANGLING_HANDLE_ERROR)
            .split_application()
            .ok_or(ErrorCode::NotADisjunction)?;

        let (constant, _tau) = self
            .term_split_constant(left)
            .map_err(|_e| ErrorCode::NotADisjunction)?;

        if constant == &PREALLOCATED_HANDLE_CONSTANT_DISJUNCTION {
            Ok((mid, right))
        } else {
            Err(ErrorCode::NotADisjunction)
        }
    }

    pub fn term_split_conjunction<T>(
        &self,
        handle: T,
    ) -> Result<(&Handle<tags::Term>, &Handle<tags::Term>), ErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        let (left, right) = self
            .resolve_term_handle(handle)?
            .split_application()
            .ok_or(ErrorCode::NotAConjunction)?;

        let (left, mid) = self
            .resolve_term_handle(left)
            .expect(DANGLING_HANDLE_ERROR)
            .split_application()
            .ok_or(ErrorCode::NotAConjunction)?;

        let (constant, _tau) = self
            .term_split_constant(left)
            .map_err(|_e| ErrorCode::NotAConjunction)?;

        if constant == &PREALLOCATED_HANDLE_CONSTANT_CONJUNCTION {
            Ok((mid, right))
        } else {
            Err(ErrorCode::NotAConjunction)
        }
    }

    pub fn term_split_implication<T>(
        &self,
        handle: T,
    ) -> Result<(&Handle<tags::Term>, &Handle<tags::Term>), ErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        let (left, right) = self
            .resolve_term_handle(handle)?
            .split_application()
            .ok_or(ErrorCode::NotAnImplication)?;

        let (left, mid) = self
            .resolve_term_handle(left)
            .expect(DANGLING_HANDLE_ERROR)
            .split_application()
            .ok_or(ErrorCode::NotAnImplication)?;

        let (constant, _tau) = self
            .term_split_constant(left)
            .map_err(|_e| ErrorCode::NotAnImplication)?;

        if constant == &PREALLOCATED_HANDLE_CONSTANT_IMPLICATION {
            Ok((mid, right))
        } else {
            Err(ErrorCode::NotAnImplication)
        }
    }

    pub fn term_split_forall<T>(
        &self,
        handle: T,
    ) -> Result<(&Name, &Handle<tags::Type>, &Handle<tags::Term>), ErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        let (left, right) = self
            .resolve_term_handle(handle)?
            .split_application()
            .ok_or(ErrorCode::NotAForall)?;

        let (constant, _tau) = self
            .term_split_constant(left)
            .map_err(|_e| ErrorCode::NotAForall)?;

        let (name, _type, body) = self
            .term_split_lambda(right)
            .map_err(|_e| ErrorCode::NotAForall)?;

        if constant == &PREALLOCATED_HANDLE_CONSTANT_FORALL {
            Ok((name, _type, body))
        } else {
            Err(ErrorCode::NotAForall)
        }
    }

    pub fn term_split_exists<T>(
        &self,
        handle: T,
    ) -> Result<(&Name, &Handle<tags::Type>, &Handle<tags::Term>), ErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        let (left, right) = self
            .resolve_term_handle(handle)?
            .split_application()
            .ok_or(ErrorCode::NotAnExists)?;

        let (constant, _tau) = self
            .term_split_constant(left)
            .map_err(|_e| ErrorCode::NotAnExists)?;

        let (name, _type, body) = self
            .term_split_lambda(right)
            .map_err(|_e| ErrorCode::NotAnExists)?;

        if constant == &PREALLOCATED_HANDLE_CONSTANT_EXISTS {
            Ok((name, _type, body))
        } else {
            Err(ErrorCode::NotAnExists)
        }
    }

    #[inline]
    pub fn term_test_variable<T>(&self, handle: T) -> Result<bool, ErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        Ok(self.term_split_variable(handle).is_ok())
    }

    #[inline]
    pub fn term_test_constant<T>(&self, handle: T) -> Result<bool, ErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        Ok(self.term_split_constant(handle).is_ok())
    }

    #[inline]
    pub fn term_test_application<T>(&self, handle: T) -> Result<bool, ErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        Ok(self.term_split_application(handle).is_ok())
    }

    #[inline]
    pub fn term_test_lambda<T>(&self, handle: T) -> Result<bool, ErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        Ok(self.term_split_lambda(handle).is_ok())
    }

    pub fn is_true<T>(&self, handle: T) -> Result<bool, ErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        let (handle, tau) = self
            .resolve_term_handle(handle)?
            .split_constant()
            .ok_or(ErrorCode::NotAConstant)?;

        Ok(handle == &PREALLOCATED_HANDLE_CONSTANT_TRUE && tau == &PREALLOCATED_HANDLE_TYPE_PROP)
    }

    pub fn is_false<T>(&self, handle: T) -> Result<bool, ErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        let (handle, tau) = self
            .resolve_term_handle(handle)?
            .split_constant()
            .ok_or(ErrorCode::NotAConstant)?;

        Ok(handle == &PREALLOCATED_HANDLE_CONSTANT_FALSE && tau == &PREALLOCATED_HANDLE_TYPE_PROP)
    }

    #[inline]
    pub fn term_test_negation<T>(&self, handle: T) -> Result<bool, ErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        Ok(self.term_split_negation(handle).is_ok())
    }

    #[inline]
    pub fn term_test_equality<T>(&self, handle: T) -> Result<bool, ErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        Ok(self.term_split_equality(handle).is_ok())
    }

    #[inline]
    pub fn term_test_disjunction<T>(&self, handle: T) -> Result<bool, ErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        Ok(self.term_split_disjunction(handle).is_ok())
    }

    #[inline]
    pub fn term_test_conjunction<T>(&self, handle: T) -> Result<bool, ErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        Ok(self.term_split_conjunction(handle).is_ok())
    }

    #[inline]
    pub fn term_test_implication<T>(&self, handle: T) -> Result<bool, ErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        Ok(self.term_split_implication(handle).is_ok())
    }

    #[inline]
    pub fn term_test_forall<T>(&self, handle: T) -> Result<bool, ErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        Ok(self.term_split_forall(handle).is_ok())
    }

    #[inline]
    pub fn term_test_exists<T>(&self, handle: T) -> Result<bool, ErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        Ok(self.term_split_exists(handle).is_ok())
    }

    /// Computes the *free type-variables* of the term pointed-to by the handle
    /// `handle` in the runtime state's term-table.
    ///
    /// # Errors
    ///
    /// Returns `Err(ErrorCode::NoSuchTermRegistered)` if `handle` does not
    /// point-to any term in the runtime state's term-table.
    pub fn term_type_variables<T>(&self, handle: T) -> Result<Vec<&Name>, ErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        let trm = self.resolve_term_handle(handle)?;

        let mut work_list = vec![trm];
        let mut ftv = vec![];

        while let Some(next) = work_list.pop() {
            match next {
                Term::Variable { tau: _type, .. } => {
                    let mut fvs = self.type_ftv(_type).expect(DANGLING_HANDLE_ERROR);
                    ftv.append(&mut fvs);
                }
                Term::Constant { tau: _type, .. } => {
                    let mut fvs = self.type_ftv(_type).expect(DANGLING_HANDLE_ERROR);
                    ftv.append(&mut fvs);
                }
                Term::Application { left, right } => {
                    let left = self.resolve_term_handle(left).expect(DANGLING_HANDLE_ERROR);
                    let right = self
                        .resolve_term_handle(right)
                        .expect(DANGLING_HANDLE_ERROR);

                    work_list.push(left);
                    work_list.push(right);
                }
                Term::Lambda {
                    tau: _type, body, ..
                } => {
                    let body = self.resolve_term_handle(body).expect(DANGLING_HANDLE_ERROR);
                    let mut fvs = self.type_ftv(_type)?;

                    ftv.append(&mut fvs);
                    work_list.push(body);
                }
            }
        }

        ftv.sort();
        ftv.dedup();

        Ok(ftv)
    }

    pub fn term_free_variables<T>(
        &self,
        handle: T,
    ) -> Result<Vec<(&Name, &Handle<tags::Type>)>, ErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        let term = self.resolve_term_handle(handle)?;

        match term {
            Term::Variable { name, tau: _type } => Ok(vec![(name, _type)]),
            Term::Constant { .. } => Ok(vec![]),
            Term::Application { left, right } => {
                let mut left = self.term_free_variables(left).expect(DANGLING_HANDLE_ERROR);
                let mut right = self
                    .term_free_variables(right)
                    .expect(DANGLING_HANDLE_ERROR);

                left.append(&mut right);

                Ok(left)
            }
            Term::Lambda {
                name,
                tau: _type,
                body,
            } => {
                let body = self.term_free_variables(body).expect(DANGLING_HANDLE_ERROR);

                Ok(body
                    .iter()
                    .filter(|v| **v != (name, _type))
                    .cloned()
                    .collect())
            }
        }
    }

    pub fn substitution<T, U, V>(
        &mut self,
        handle: T,
        sigma: Vec<(U, V)>,
    ) -> Result<Handle<tags::Term>, ErrorCode>
    where
        T: Into<Handle<tags::Term>>,
        U: Into<Name> + Clone,
        V: Into<Handle<tags::Term>> + Clone,
    {
        unimplemented!()
    }

    pub fn term_type_substitution<T, U, V>(
        &mut self,
        handle: T,
        sigma: Vec<(U, V)>,
    ) -> Result<Handle<tags::Term>, ErrorCode>
    where
        T: Into<Handle<tags::Term>>,
        U: Into<Name> + Clone,
        V: Into<Handle<tags::Type>> + Clone,
    {
        unimplemented!()
    }

    pub fn term_type_infer<T>(&mut self, handle: T) -> Result<Handle<tags::Type>, ErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        let trm = self.resolve_term_handle(handle)?;

        let trm = trm.clone();

        match trm {
            Term::Variable { tau: _type, .. } => Ok(_type.clone()),
            Term::Constant { tau: _type, .. } => Ok(_type.clone()),
            Term::Application { left, right } => {
                let ltau = self.term_type_infer(&left)?;
                let rtau = self.term_type_infer(&right)?;

                let (dom, rng) = self
                    .type_split_function(&ltau)
                    .map_err(|_e| ErrorCode::NotAFunctionType)?;

                if dom == &rtau {
                    Ok(rng.clone())
                } else {
                    Err(ErrorCode::DomainTypeMismatch)
                }
            }
            Term::Lambda {
                tau: _type, body, ..
            } => {
                let btau = self.term_type_infer(&body)?;
                Ok(self.admit_type(Type::function(_type, btau)))
            }
        }
    }

    /// Returns `Ok(true)` iff the type of the term pointed-to by `handle` in
    /// the runtime state's term-table has propositional type.
    ///
    /// # Errors
    ///
    /// Returns `Err(ErrorCode::NoSuchTermRegistered)` if `handle` does not
    /// point-to a term in the runtime state's term-table.
    ///
    /// Returns `Err(ErrorCode::DomainTypeMismatch)` if the term pointed-to by
    /// `handle` is not typeable.
    #[inline]
    pub fn term_type_is_proposition<T>(&mut self, handle: T) -> Result<bool, ErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        Ok(self.term_type_infer(handle)? == PREALLOCATED_HANDLE_TYPE_PROP)
    }

    fn swap<T, U, V>(&mut self, handle: T, a: U, b: V) -> Result<Handle<tags::Term>, ErrorCode>
    where
        T: Borrow<Handle<tags::Term>> + Clone,
        U: Into<Name> + Clone,
        V: Into<Name> + Clone,
    {
        let trm = self.resolve_term_handle(handle.clone())?.clone();

        match trm {
            Term::Variable { name, tau: _type } => {
                if name == a.clone().into() {
                    Ok(self.admit_term(Term::variable(b, _type.clone())))
                } else if name == b.into() {
                    Ok(self.admit_term(Term::variable(a, _type.clone())))
                } else {
                    Ok(handle.borrow().clone())
                }
            }
            Term::Constant { .. } => Ok(handle.clone().borrow().clone()),
            Term::Application { left, right } => {
                let left = self
                    .swap(&left, a.clone(), b.clone())
                    .expect(DANGLING_HANDLE_ERROR);
                let right = self.swap(&right, a, b).expect(DANGLING_HANDLE_ERROR);

                Ok(self.admit_term(Term::application(left, right)))
            }
            Term::Lambda {
                name,
                tau: _type,
                body,
            } => {
                let body = self
                    .swap(&body, a.clone(), b.clone())
                    .expect(DANGLING_HANDLE_ERROR);
                if name == a.clone().into() {
                    Ok(self.admit_term(Term::lambda(b, _type.clone(), body)))
                } else if name == b.into() {
                    Ok(self.admit_term(Term::lambda(a, _type.clone(), body)))
                } else {
                    Ok(self.admit_term(Term::lambda(name, _type.clone(), body)))
                }
            }
        }
    }

    fn is_alpha_equivalent_inner(&mut self, left: &Term, right: &Term) -> Result<bool, ErrorCode> {
        match (left, right) {
            (
                Term::Variable {
                    name: name0,
                    tau: _type0,
                },
                Term::Variable {
                    name: name1,
                    tau: _type1,
                },
            ) => Ok(name0 == name1 && _type0 == _type1),
            (
                Term::Constant {
                    constant: handle0,
                    tau: _type0,
                },
                Term::Constant {
                    constant: handle1,
                    tau: _type1,
                },
            ) => Ok(handle0 == handle1 && _type0 == _type1),
            (
                Term::Application {
                    left: left0,
                    right: right0,
                },
                Term::Application {
                    left: left1,
                    right: right1,
                },
            ) => {
                let left = self
                    .is_alpha_equivalent(left0, left1)
                    .expect(DANGLING_HANDLE_ERROR);
                let right = self
                    .is_alpha_equivalent(right0, right1)
                    .expect(DANGLING_HANDLE_ERROR);
                Ok(left && right)
            }
            (
                Term::Lambda {
                    name: name0,
                    tau: _type0,
                    body: body0,
                },
                Term::Lambda {
                    name: name1,
                    tau: _type1,
                    body: body1,
                },
            ) => {
                if name0 == name1 {
                    let body = self.is_alpha_equivalent(body0, body1)?;
                    Ok(body && _type0 == _type1)
                } else if !self
                    .term_free_variables(body1)
                    .expect(DANGLING_HANDLE_ERROR)
                    .contains(&(name0, _type0))
                {
                    let body1 = self
                        .swap(body1, name0.clone(), name1.clone())
                        .expect(DANGLING_HANDLE_ERROR);
                    let body = self
                        .is_alpha_equivalent(body0, &body1)
                        .expect(DANGLING_HANDLE_ERROR);

                    Ok(body && _type0 == _type1)
                } else {
                    Ok(false)
                }
            }
            _otherwise => Ok(false),
        }
    }

    #[inline]
    pub fn is_alpha_equivalent<T>(&mut self, left: T, right: T) -> Result<bool, ErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        let left = self.resolve_term_handle(left)?.clone();
        let right = self.resolve_term_handle(right)?.clone();

        self.is_alpha_equivalent_inner(&left, &right)
    }

    ////////////////////////////////////////////////////////////////////////////
    // Theorem related material.
    ////////////////////////////////////////////////////////////////////////////

    /// Admits a new theorem `thm` into the runtime state's theorem-table.  If
    /// an existing theorem has been registered in the theorem-table that is
    /// alpha-equivalent to `thm` then the handle associated with this existing
    /// theorem is returned.  Otherwise, a fresh handle is generated and `thm`
    /// is associated with this theorem.
    ///
    /// Callers are expected to:
    /// 1. Ensure that `thm` is well-formed before calling this function,
    /// 2. The hypotheses of the theorem `thm` should be sorted prior to calling
    /// this function, so that theorems can be compared for structural equality.
    fn admit_theorem(&mut self, thm: Theorem) -> Handle<tags::Theorem> {
        let fresh = self.issue_handle();
        self.theorems.insert(fresh.clone(), thm);
        fresh
    }

    /// Returns `Some(thm)` iff `handle` points-to a registered theorem in the
    /// runtime state's theorem table.
    #[inline]
    pub fn resolve_theorem_handle<T>(&self, handle: T) -> Option<&Theorem>
    where
        T: Borrow<Handle<tags::Theorem>>,
    {
        self.theorems.get(handle.borrow())
    }

    /// Returns `true` iff `handle` points to a registered theorem in the
    /// runtime state's theorem table.
    #[inline]
    pub fn is_theorem_registered<T>(&self, handle: T) -> bool
    where
        T: Borrow<Handle<tags::Theorem>>,
    {
        self.resolve_theorem_handle(handle).is_some()
    }

    pub fn register_reflexivity_theorem<T, U>(
        &mut self,
        hypotheses: Vec<T>,
        trm: U,
    ) -> Result<Handle<tags::Theorem>, ErrorCode>
    where
        T: Into<Handle<tags::Term>> + Clone,
        U: Into<Handle<tags::Term>> + Clone,
    {
        if !self.term_type_is_proposition(trm.clone().into())? {
            return Err(ErrorCode::NotAProposition);
        }

        for c in hypotheses.iter().cloned() {
            if !self.term_type_is_proposition(&c.into())? {
                return Err(ErrorCode::NotAProposition);
            }
        }

        let conclusion = self.term_register_equality(trm.clone(), trm)?;

        Ok(self.admit_theorem(Theorem::new(hypotheses, conclusion)))
    }

    pub fn register_symmetry_theorem<T>(
        &mut self,
        handle: T,
    ) -> Result<Handle<tags::Theorem>, ErrorCode>
    where
        T: Borrow<Handle<tags::Theorem>>,
    {
        let thm = self
            .resolve_theorem_handle(handle)
            .ok_or(ErrorCode::NoSuchTheoremRegistered)?
            .clone();

        let (left, right) = self.term_split_equality(thm.conclusion())?;

        let left = left.clone();
        let right = right.clone();

        let conclusion = self.term_register_equality(right, left)?;
        let hypotheses = thm.hypotheses().clone();

        Ok(self.admit_theorem(Theorem::new(hypotheses, conclusion)))
    }

    pub fn register_transitivity_theorem<T>(
        &mut self,
        left: T,
        right: T,
    ) -> Result<Handle<tags::Theorem>, ErrorCode>
    where
        T: Borrow<Handle<tags::Theorem>>,
    {
        let left = self
            .resolve_theorem_handle(left)
            .ok_or(ErrorCode::NoSuchTheoremRegistered)?
            .clone();
        let right = self
            .resolve_theorem_handle(right)
            .ok_or(ErrorCode::NoSuchTheoremRegistered)?
            .clone();

        if left.hypotheses() != right.hypotheses() {
            return Err(ErrorCode::ShapeMismatch);
        }

        let hypotheses = left.hypotheses().clone();

        let (left, mid0) = self.term_split_equality(left.conclusion())?;
        let (mid1, right) = self.term_split_equality(right.conclusion())?;

        if mid0 != mid1 {
            return Err(ErrorCode::ShapeMismatch);
        }

        let left = left.clone();
        let right = right.clone();

        let conclusion = self.term_register_equality(left, right)?;

        Ok(self.admit_theorem(Theorem::new(hypotheses, conclusion)))
    }

    pub fn register_application_congruence_theorem<T>(
        &mut self,
        left: T,
        right: T,
    ) -> Result<Handle<tags::Theorem>, ErrorCode>
    where
        T: Borrow<Handle<tags::Theorem>>,
    {
        let left = self
            .resolve_theorem_handle(left)
            .ok_or(ErrorCode::NoSuchTheoremRegistered)?
            .clone();
        let right = self
            .resolve_theorem_handle(right)
            .ok_or(ErrorCode::NoSuchTheoremRegistered)?
            .clone();

        let mut hypotheses = left.hypotheses().clone();
        let mut merge = right.hypotheses().clone();

        hypotheses.append(&mut merge);

        let (fun_left, fun_right) = self.term_split_equality(left.conclusion())?;
        let (arg_left, arg_right) = self.term_split_equality(right.conclusion())?;

        let fun_left = fun_left.clone();
        let fun_right = fun_right.clone();
        let arg_left = arg_left.clone();
        let arg_right = arg_right.clone();

        let left = self.term_register_application(fun_left, arg_left)?;
        let right = self.term_register_application(fun_right, arg_right)?;

        let hypotheses = hypotheses.clone();
        let conclusion = self.term_register_equality(left, right)?;

        Ok(self.admit_theorem(Theorem::new(hypotheses, conclusion)))
    }

    pub fn register_lambda_congruence_theorem<T, U, V>(
        &mut self,
        name: T,
        tau: U,
        handle: V,
    ) -> Result<Handle<tags::Theorem>, ErrorCode>
    where
        T: Into<Name> + Clone,
        U: Into<Handle<tags::Type>> + Clone,
        V: Borrow<Handle<tags::Theorem>>,
    {
        if !self.type_is_registered(tau.clone().into()) {
            return Err(ErrorCode::NoSuchTypeRegistered);
        }

        let thm = self
            .resolve_theorem_handle(handle)
            .ok_or(ErrorCode::NoSuchTheoremRegistered)?
            .clone();

        let (left, right) = self.term_split_equality(thm.conclusion())?;

        let left = left.clone();
        let right = right.clone();

        let lhandle = self.term_register_lambda(name.clone(), tau.clone(), left)?;
        let rhandle = self.term_register_lambda(name, tau, right)?;

        let conclusion = self.term_register_equality(lhandle, rhandle)?;
        let hypotheses = thm.hypotheses().clone();

        Ok(self.admit_theorem(Theorem::new(hypotheses, conclusion)))
    }

    pub fn register_beta_theorem<T>(
        &mut self,
        hypotheses: Vec<T>,
        application: T,
    ) -> Result<Handle<tags::Theorem>, ErrorCode>
    where
        T: Into<Handle<tags::Term>> + Clone,
    {
        for c in hypotheses.iter().cloned() {
            if !self.term_type_is_proposition(c.into())? {
                return Err(ErrorCode::NotAProposition);
            }
        }

        let (lhs, rhs) = self.term_split_application(application.clone().into())?;

        let lhs = lhs.clone();
        let rhs = rhs.clone();

        let (name, _type, body) = self.term_split_lambda(&lhs)?;

        let name = name.clone();
        let rhs = rhs.clone();
        let body = body.clone();

        let subst = self.substitution(body, vec![(name, rhs)])?;
        let conclusion = self.term_register_equality(application, subst)?;

        Ok(self.admit_theorem(Theorem::new(hypotheses, conclusion)))
    }

    pub fn register_eta_theorem<T>(
        &mut self,
        hypotheses: Vec<T>,
        lambda: T,
    ) -> Result<Handle<tags::Theorem>, ErrorCode>
    where
        T: Into<Handle<tags::Term>> + Clone,
    {
        for c in hypotheses.iter().cloned() {
            if !self.term_type_is_proposition(c.into())? {
                return Err(ErrorCode::NotAProposition);
            }
        }

        let (name0, _type, body) = self.term_split_lambda(lambda.clone().into())?;
        let (func, var) = self.term_split_application(body)?;

        let (name1, _type) = self.term_split_variable(var)?;

        if name0 != name1 {
            return Err(ErrorCode::ShapeMismatch);
        }

        if self.term_free_variables(func)?.contains(&(name1, _type)) {
            return Err(ErrorCode::ShapeMismatch);
        }

        let body = body.clone();

        let conclusion = self.term_register_equality(lambda, body)?;

        Ok(self.admit_theorem(Theorem::new(hypotheses, conclusion)))
    }

    pub fn register_equality_introduction_theorem<T>(
        &mut self,
        left: T,
        right: T,
    ) -> Result<Handle<tags::Theorem>, ErrorCode>
    where
        T: Borrow<Handle<tags::Theorem>>,
    {
        let left = self
            .resolve_theorem_handle(left)
            .ok_or(ErrorCode::NoSuchTheoremRegistered)?
            .clone();
        let right = self
            .resolve_theorem_handle(right)
            .ok_or(ErrorCode::NoSuchTheoremRegistered)?
            .clone();

        let (left0, right0) = self.term_split_implication(left.conclusion())?;
        let (left1, right1) = self.term_split_implication(right.conclusion())?;

        if left0 != right1 || left1 != right0 {
            return Err(ErrorCode::ShapeMismatch);
        }

        let left0 = left0.clone();
        let right1 = right1.clone();

        let mut hypotheses = left.hypotheses().clone();
        let mut merge = right.hypotheses().clone();

        hypotheses.append(&mut merge);

        let conclusion = self.term_register_equality(left0, right1)?;

        Ok(self.admit_theorem(Theorem::new(hypotheses, conclusion)))
    }

    pub fn register_equality_elimination_theorem<T>(
        &mut self,
        handle: T,
    ) -> Result<Handle<tags::Theorem>, ErrorCode>
    where
        T: Borrow<Handle<tags::Theorem>>,
    {
        let thm = self
            .resolve_theorem_handle(handle)
            .ok_or(ErrorCode::NoSuchTheoremRegistered)?
            .clone();

        let (left, right) = self
            .term_split_equality(thm.conclusion())
            .map_err(|_e| ErrorCode::ShapeMismatch)?;

        let left = left.clone();
        let right = right.clone();

        if !self.term_type_is_proposition(&left)? {
            return Err(ErrorCode::NotAProposition);
        }

        let conclusion = self.term_register_implication(left, right)?;
        let hypotheses = thm.hypotheses().clone();

        Ok(self.admit_theorem(Theorem::new(hypotheses, conclusion)))
    }

    pub fn register_truth_introduction_theorem<T>(
        &mut self,
        hypotheses: Vec<T>,
    ) -> Result<Handle<tags::Theorem>, ErrorCode>
    where
        T: Into<Handle<tags::Term>> + Clone,
    {
        for c in hypotheses.iter().cloned() {
            if !self.is_term_registered(c.into()) {
                return Err(ErrorCode::NoSuchTermRegistered);
            }
        }

        let empty: Vec<(Name, Handle<tags::Type>)> = Vec::new();

        let conclusion = self.term_register_constant(PREALLOCATED_HANDLE_CONSTANT_TRUE, empty)?;

        Ok(self.admit_theorem(Theorem::new(hypotheses, conclusion)))
    }

    pub fn register_false_elimination_theorem<T, U>(
        &mut self,
        thm: T,
        conclusion: U,
    ) -> Result<Handle<tags::Theorem>, ErrorCode>
    where
        T: Borrow<Handle<tags::Theorem>>,
        U: Into<Handle<tags::Term>> + Clone,
    {
        let thm = self
            .resolve_theorem_handle(thm)
            .ok_or(ErrorCode::NoSuchTheoremRegistered)?
            .clone();

        if !self.is_term_registered(conclusion.clone().into()) {
            return Err(ErrorCode::NoSuchTermRegistered);
        }

        if !self.term_type_is_proposition(conclusion.clone().into())? {
            return Err(ErrorCode::NotAProposition);
        }

        if !self.is_false(thm.conclusion())? {
            return Err(ErrorCode::ShapeMismatch);
        }

        let hypotheses = thm.hypotheses().clone();

        Ok(self.admit_theorem(Theorem::new(hypotheses, conclusion)))
    }

    pub fn register_conjunction_introduction_theorem<T>(
        &mut self,
        left: T,
        right: T,
    ) -> Result<Handle<tags::Theorem>, ErrorCode>
    where
        T: Borrow<Handle<tags::Theorem>>,
    {
        let left = self
            .resolve_theorem_handle(left)
            .ok_or(ErrorCode::NoSuchTheoremRegistered)?
            .clone();
        let right = self
            .resolve_theorem_handle(right)
            .ok_or(ErrorCode::NoSuchTheoremRegistered)?
            .clone();

        let conclusion =
            self.term_register_conjunction(left.conclusion().clone(), right.conclusion().clone())?;

        let mut hypotheses = left.hypotheses().clone();
        let mut merge = right.hypotheses().clone();

        hypotheses.append(&mut merge);

        Ok(self.admit_theorem(Theorem::new(hypotheses, conclusion)))
    }

    pub fn register_conjunction_elimination0_theorem<T>(
        &mut self,
        handle: T,
    ) -> Result<Handle<tags::Theorem>, ErrorCode>
    where
        T: Borrow<Handle<tags::Theorem>>,
    {
        let thm = self
            .resolve_theorem_handle(handle)
            .ok_or(ErrorCode::NoSuchTheoremRegistered)?
            .clone();

        let (left, _right) = self
            .term_split_conjunction(thm.conclusion())
            .map_err(|_e| ErrorCode::ShapeMismatch)?;

        let conclusion = left.clone();
        let hypotheses = thm.hypotheses().clone();

        Ok(self.admit_theorem(Theorem::new(hypotheses, conclusion)))
    }

    pub fn register_conjunction_elimination1_theorem<T>(
        &mut self,
        handle: T,
    ) -> Result<Handle<tags::Theorem>, ErrorCode>
    where
        T: Borrow<Handle<tags::Theorem>>,
    {
        let thm = self
            .resolve_theorem_handle(handle)
            .ok_or(ErrorCode::NoSuchTheoremRegistered)?
            .clone();

        let (_left, right) = self
            .term_split_conjunction(thm.conclusion())
            .map_err(|_e| ErrorCode::ShapeMismatch)?;

        let conclusion = right.clone();
        let hypotheses = thm.hypotheses().clone();

        Ok(self.admit_theorem(Theorem::new(hypotheses, conclusion)))
    }

    pub fn register_disjunction_introduction0_theorem<T, U>(
        &mut self,
        handle: T,
        term: U,
    ) -> Result<Handle<tags::Theorem>, ErrorCode>
    where
        T: Borrow<Handle<tags::Theorem>>,
        U: Into<Handle<tags::Term>> + Clone,
    {
        let thm = self
            .resolve_theorem_handle(handle)
            .ok_or(ErrorCode::NoSuchTheoremRegistered)?
            .clone();

        if !self.term_type_is_proposition(term.clone().into())? {
            return Err(ErrorCode::NotAProposition);
        }

        let conclusion = self.term_register_disjunction(thm.conclusion().clone(), term)?;
        let hypotheses = thm.hypotheses().clone();

        Ok(self.admit_theorem(Theorem::new(hypotheses, conclusion)))
    }

    pub fn register_disjunction_introduction1_theorem<T, U>(
        &mut self,
        handle: T,
        term: U,
    ) -> Result<Handle<tags::Theorem>, ErrorCode>
    where
        T: Borrow<Handle<tags::Theorem>>,
        U: Into<Handle<tags::Term>> + Clone,
    {
        let thm = self
            .resolve_theorem_handle(handle)
            .ok_or(ErrorCode::NoSuchTheoremRegistered)?
            .clone();

        if !self.term_type_is_proposition(term.clone().into())? {
            return Err(ErrorCode::NotAProposition);
        }

        let conclusion = self.term_register_disjunction(term, thm.conclusion().clone())?;
        let hypotheses = thm.hypotheses().clone();

        Ok(self.admit_theorem(Theorem::new(hypotheses, conclusion)))
    }

    pub fn register_disjunction_elimination_theorem<T>(
        &mut self,
        left: T,
        mid: T,
        right: T,
    ) -> Result<Handle<tags::Theorem>, ErrorCode>
    where
        T: Borrow<Handle<tags::Theorem>>,
    {
        let left = self
            .resolve_theorem_handle(left)
            .ok_or(ErrorCode::NoSuchTheoremRegistered)?
            .clone();
        let mid = self
            .resolve_theorem_handle(mid)
            .ok_or(ErrorCode::NoSuchTheoremRegistered)?
            .clone();
        let right = self
            .resolve_theorem_handle(right)
            .ok_or(ErrorCode::NoSuchTheoremRegistered)?
            .clone();

        let (phi, psi) = self.term_split_disjunction(left.conclusion())?;

        if mid.conclusion() != right.conclusion() {
            return Err(ErrorCode::ShapeMismatch);
        }

        if !mid.hypotheses().contains(phi) || !right.hypotheses().contains(psi) {
            return Err(ErrorCode::ShapeMismatch);
        }

        if left.hypotheses().clone()
            != mid
                .hypotheses()
                .iter()
                .filter(|h| *h != phi)
                .cloned()
                .collect::<Vec<_>>()
            || left.hypotheses().clone()
                != right
                    .hypotheses()
                    .iter()
                    .filter(|h| *h != psi)
                    .cloned()
                    .collect::<Vec<_>>()
        {
            return Err(ErrorCode::ShapeMismatch);
        }

        let conclusion = right.conclusion().clone();
        let hypotheses = left.hypotheses().clone();

        Ok(self.admit_theorem(Theorem::new(hypotheses, conclusion)))
    }

    pub fn register_implication_introduction_theorem<T, U>(
        &mut self,
        handle: T,
        intro: U,
    ) -> Result<Handle<tags::Theorem>, ErrorCode>
    where
        T: Borrow<Handle<tags::Theorem>>,
        U: Into<Handle<tags::Term>> + Clone,
    {
        let thm = self
            .resolve_theorem_handle(handle)
            .ok_or(ErrorCode::NoSuchTheoremRegistered)?
            .clone();

        if !self.term_type_is_proposition(intro.clone().into())? {
            return Err(ErrorCode::NotAProposition);
        }

        if !thm.hypotheses().contains(&intro.clone().into()) {
            return Err(ErrorCode::ShapeMismatch);
        }

        let conclusion = self.term_register_implication(intro.clone(), thm.conclusion().clone())?;
        let hypotheses = thm
            .hypotheses()
            .iter()
            .filter(|h| **h != intro.clone().into())
            .cloned()
            .collect();

        Ok(self.admit_theorem(Theorem::new(hypotheses, conclusion)))
    }

    pub fn register_implication_elimination_theorem<T>(
        &mut self,
        left: T,
        right: T,
    ) -> Result<Handle<tags::Theorem>, ErrorCode>
    where
        T: Borrow<Handle<tags::Theorem>>,
    {
        let left = self
            .resolve_theorem_handle(left)
            .ok_or(ErrorCode::NoSuchTheoremRegistered)?
            .clone();
        let right = self
            .resolve_theorem_handle(right)
            .ok_or(ErrorCode::NoSuchTheoremRegistered)?
            .clone();

        let (hyp, conc) = self
            .term_split_implication(left.conclusion())
            .map_err(|_e| ErrorCode::ShapeMismatch)?;

        if hyp != right.conclusion() {
            return Err(ErrorCode::ShapeMismatch);
        }

        let conc = conc.clone();

        let mut hypotheses = left.hypotheses().clone();
        let mut merge = right.hypotheses().clone();

        hypotheses.append(&mut merge);

        Ok(self.admit_theorem(Theorem::new(hypotheses, conc)))
    }

    pub fn register_substitution_theorem<T, U>(
        &mut self,
        handle: T,
        sigma: Vec<(Name, U)>,
    ) -> Result<Handle<tags::Theorem>, ErrorCode>
    where
        T: Borrow<Handle<tags::Theorem>>,
        U: Into<Handle<tags::Term>> + Clone,
    {
        let thm = self
            .resolve_theorem_handle(handle)
            .ok_or(ErrorCode::NoSuchTheoremRegistered)?
            .clone();

        let conclusion = self.substitution(thm.conclusion().clone(), sigma.clone())?;
        let mut hypotheses = vec![];

        for h in thm.hypotheses().iter().cloned() {
            hypotheses.push(self.substitution(h, sigma.clone())?);
        }

        Ok(self.admit_theorem(Theorem::new(hypotheses, conclusion)))
    }

    pub fn register_instantiation_theorem<T, U>(
        &mut self,
        handle: T,
        sigma: Vec<(Name, U)>,
    ) -> Result<Handle<tags::Theorem>, ErrorCode>
    where
        T: Borrow<Handle<tags::Theorem>>,
        U: Into<Handle<tags::Type>> + Clone,
    {
        let thm = self
            .resolve_theorem_handle(handle)
            .ok_or(ErrorCode::NoSuchTheoremRegistered)?
            .clone();

        let conclusion = self.term_type_substitution(thm.conclusion().clone(), sigma.clone())?;
        let mut hypotheses = Vec::new();

        for h in thm.hypotheses().iter().cloned() {
            hypotheses.push(self.term_type_substitution(h, sigma.clone())?);
        }

        Ok(self.admit_theorem(Theorem::new(hypotheses, conclusion)))
    }

    pub fn register_universal_elimination_theorem<T, U>(
        &mut self,
        handle: T,
        trm: U,
    ) -> Result<Handle<tags::Theorem>, ErrorCode>
    where
        T: Borrow<Handle<tags::Theorem>>,
        U: Into<Handle<tags::Term>>,
    {
        unimplemented!()
    }

    pub fn register_universal_introduction_theorem<T, U>(
        &mut self,
        handle: T,
        name: U,
    ) -> Result<Handle<tags::Theorem>, ErrorCode>
    where
        T: Borrow<Handle<tags::Theorem>>,
        U: Into<Name>,
    {
        unimplemented!()
    }

    pub fn register_existential_introduction_theorem<T, U>(
        &mut self,
        handle: T,
        trm: U,
    ) -> Result<Handle<tags::Theorem>, ErrorCode>
    where
        T: Borrow<Handle<tags::Theorem>>,
        U: Into<Handle<tags::Term>>,
    {
        unimplemented!()
    }

    ////////////////////////////////////////////////////////////////////////////
    // Modifying the global theory.
    ////////////////////////////////////////////////////////////////////////////

    pub fn register_new_definition<T>(
        &mut self,
        defn: T,
    ) -> Result<(Handle<tags::Term>, Handle<tags::Theorem>), ErrorCode>
    where
        T: Into<Handle<tags::Term>> + Clone,
    {
        /* 1. Check the body of the definition exists, and it has a type. */
        let tau = self.term_type_infer(defn.clone().into())?;

        /* 2. Add the new constant, giving it the type inferred previously. */
        let cnst_handle = self.issue_handle();
        self.constants.insert(cnst_handle.clone(), tau.clone());

        let empty: Vec<(Name, Handle<tags::Type>)> = Vec::new();

        /* 3. Lift the registered constant into a term. */
        let cnst = self
            .term_register_constant(cnst_handle, empty)
            .expect(PRIMITIVE_CONSTRUCTION_ERROR);

        /* 4. Construct the definitional theorem. */
        let stmt = self
            .term_register_equality(cnst.clone(), defn.clone())
            .expect(PRIMITIVE_CONSTRUCTION_ERROR);

        /* 5. Register the definitional theorem. */
        let empty: Vec<Handle<tags::Term>> = Vec::new();
        let thm = self.admit_theorem(Theorem::new(empty, stmt));

        /* 6. Return the handle to the new constant and definitional theorem. */

        Ok((cnst, thm))
    }
}

impl Default for RuntimeState {
    /// Creates a new, default runtime state with all primitive kernel objects
    /// registered and the next handle counter suitably initialized.
    fn default() -> RuntimeState {
        let type_formers = HashMap::from_iter(vec![
            (PREALLOCATED_HANDLE_TYPE_FORMER_PROP, 0),
            (PREALLOCATED_HANDLE_TYPE_FORMER_ARROW, 2),
        ]);

        let types = HashMap::from_iter(vec![
            (PREALLOCATED_HANDLE_TYPE_ALPHA, TYPE_ALPHA.clone()),
            (PREALLOCATED_HANDLE_TYPE_BETA, TYPE_BETA.clone()),
            (PREALLOCATED_HANDLE_TYPE_PROP, TYPE_PROP.clone()),
            (
                PREALLOCATED_HANDLE_TYPE_UNARY_CONNECTIVE,
                TYPE_UNARY_CONNECTIVE.clone(),
            ),
            (
                PREALLOCATED_HANDLE_TYPE_BINARY_CONNECTIVE,
                TYPE_BINARY_CONNECTIVE.clone(),
            ),
            (
                PREALLOCATED_HANDLE_TYPE_UNARY_PREDICATE,
                TYPE_POLYMORPHIC_UNARY_PREDICATE.clone(),
            ),
            (
                PREALLOCATED_HANDLE_TYPE_BINARY_PREDICATE,
                TYPE_POLYMORPHIC_BINARY_PREDICATE.clone(),
            ),
            (
                PREALLOCATED_HANDLE_TYPE_QUANTIFIER,
                TYPE_POLYMORPHIC_QUANTIFIER.clone(),
            ),
        ]);

        let constants = HashMap::from_iter(vec![
            (
                PREALLOCATED_HANDLE_CONSTANT_TRUE,
                PREALLOCATED_HANDLE_TYPE_PROP,
            ),
            (
                PREALLOCATED_HANDLE_CONSTANT_FALSE,
                PREALLOCATED_HANDLE_TYPE_PROP,
            ),
            (
                PREALLOCATED_HANDLE_CONSTANT_NEGATION,
                PREALLOCATED_HANDLE_TYPE_UNARY_CONNECTIVE,
            ),
            (
                PREALLOCATED_HANDLE_CONSTANT_CONJUNCTION,
                PREALLOCATED_HANDLE_TYPE_BINARY_CONNECTIVE,
            ),
            (
                PREALLOCATED_HANDLE_CONSTANT_DISJUNCTION,
                PREALLOCATED_HANDLE_TYPE_BINARY_CONNECTIVE,
            ),
            (
                PREALLOCATED_HANDLE_CONSTANT_IMPLICATION,
                PREALLOCATED_HANDLE_TYPE_BINARY_CONNECTIVE,
            ),
            (
                PREALLOCATED_HANDLE_CONSTANT_EQUALITY,
                PREALLOCATED_HANDLE_TYPE_BINARY_PREDICATE,
            ),
            (
                PREALLOCATED_HANDLE_CONSTANT_FORALL,
                PREALLOCATED_HANDLE_TYPE_QUANTIFIER,
            ),
            (
                PREALLOCATED_HANDLE_CONSTANT_EXISTS,
                PREALLOCATED_HANDLE_TYPE_QUANTIFIER,
            ),
        ]);

        let terms = HashMap::from_iter(vec![
            (PREALLOCATED_HANDLE_TERM_TRUE, TERM_TRUE_CONSTANT),
            (PREALLOCATED_HANDLE_TERM_FALSE, TERM_FALSE_CONSTANT),
            (PREALLOCATED_HANDLE_TERM_FORALL, TERM_FORALL_CONSTANT),
            (PREALLOCATED_HANDLE_TERM_EXISTS, TERM_EXISTS_CONSTANT),
            (PREALLOCATED_HANDLE_TERM_NEGATION, TERM_NEGATION_CONSTANT),
            (
                PREALLOCATED_HANDLE_TERM_IMPLICATION,
                TERM_IMPLICATION_CONSTANT,
            ),
            (
                PREALLOCATED_HANDLE_TERM_CONJUNCTION,
                TERM_CONJUNCTION_CONSTANT,
            ),
            (
                PREALLOCATED_HANDLE_TERM_DISJUNCTION,
                TERM_DISJUNCTION_CONSTANT,
            ),
            (PREALLOCATED_HANDLE_TERM_EQUALITY, TERM_EQUALITY_CONSTANT),
        ]);

        let theorems = HashMap::from_iter(vec![]);

        RuntimeState {
            next_handle: PREALLOCATED_HANDLE_UPPER_BOUND,
            type_formers,
            types,
            constants,
            terms,
            theorems,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::kernel::{
        handle::{
            PREALLOCATED_HANDLE_CONSTANT_CONJUNCTION, PREALLOCATED_HANDLE_CONSTANT_DISJUNCTION,
            PREALLOCATED_HANDLE_CONSTANT_EQUALITY, PREALLOCATED_HANDLE_CONSTANT_EXISTS,
            PREALLOCATED_HANDLE_CONSTANT_FALSE, PREALLOCATED_HANDLE_CONSTANT_FORALL,
            PREALLOCATED_HANDLE_CONSTANT_IMPLICATION, PREALLOCATED_HANDLE_CONSTANT_NEGATION,
            PREALLOCATED_HANDLE_CONSTANT_TRUE, PREALLOCATED_HANDLE_TERM_CONJUNCTION,
            PREALLOCATED_HANDLE_TERM_DISJUNCTION, PREALLOCATED_HANDLE_TERM_EQUALITY,
            PREALLOCATED_HANDLE_TERM_EXISTS, PREALLOCATED_HANDLE_TERM_FALSE,
            PREALLOCATED_HANDLE_TERM_FORALL, PREALLOCATED_HANDLE_TERM_IMPLICATION,
            PREALLOCATED_HANDLE_TERM_NEGATION, PREALLOCATED_HANDLE_TERM_TRUE,
            PREALLOCATED_HANDLE_TYPE_ALPHA, PREALLOCATED_HANDLE_TYPE_BETA,
            PREALLOCATED_HANDLE_TYPE_BINARY_CONNECTIVE, PREALLOCATED_HANDLE_TYPE_BINARY_PREDICATE,
            PREALLOCATED_HANDLE_TYPE_FORMER_ARROW, PREALLOCATED_HANDLE_TYPE_FORMER_PROP,
            PREALLOCATED_HANDLE_TYPE_PROP, PREALLOCATED_HANDLE_TYPE_QUANTIFIER,
            PREALLOCATED_HANDLE_TYPE_UNARY_CONNECTIVE, PREALLOCATED_HANDLE_TYPE_UNARY_PREDICATE,
        },
        runtime_state::RuntimeState,
    };

    ////////////////////////////////////////////////////////////////////////////
    // Initial theory tests.
    ////////////////////////////////////////////////////////////////////////////

    /// Tests all primitive type-formers are registered in the initial theory.
    #[test]
    pub fn initial_theory0() {
        let state = RuntimeState::new();

        assert!(state
            .type_former_resolve(&PREALLOCATED_HANDLE_TYPE_FORMER_PROP)
            .is_some());
        assert!(state
            .type_former_resolve(&PREALLOCATED_HANDLE_TYPE_FORMER_ARROW)
            .is_some());
    }

    /// Tests all primitive constants are registered in the initial theory.
    #[test]
    pub fn initial_theory1() {
        let state = RuntimeState::new();

        assert!(state
            .constant_resolve(&PREALLOCATED_HANDLE_CONSTANT_EXISTS)
            .is_ok());
        assert!(state
            .constant_resolve(&PREALLOCATED_HANDLE_CONSTANT_FORALL)
            .is_ok());
        assert!(state
            .constant_resolve(&PREALLOCATED_HANDLE_CONSTANT_IMPLICATION)
            .is_ok());
        assert!(state
            .constant_resolve(&PREALLOCATED_HANDLE_CONSTANT_CONJUNCTION)
            .is_ok());
        assert!(state
            .constant_resolve(&PREALLOCATED_HANDLE_CONSTANT_DISJUNCTION)
            .is_ok());
        assert!(state
            .constant_resolve(&PREALLOCATED_HANDLE_CONSTANT_TRUE)
            .is_ok());
        assert!(state
            .constant_resolve(&PREALLOCATED_HANDLE_CONSTANT_FALSE)
            .is_ok());
        assert!(state
            .constant_resolve(&PREALLOCATED_HANDLE_CONSTANT_EQUALITY)
            .is_ok());
        assert!(state
            .constant_resolve(&PREALLOCATED_HANDLE_CONSTANT_NEGATION)
            .is_ok());
    }

    /// Tests all primitive types are registered in the initial theory.
    #[test]
    pub fn initial_theory2() {
        let state = RuntimeState::new();

        assert!(state
            .resolve_type_handle(&PREALLOCATED_HANDLE_TYPE_PROP)
            .is_some());
        assert!(state
            .resolve_type_handle(&PREALLOCATED_HANDLE_TYPE_BINARY_PREDICATE)
            .is_some());
        assert!(state
            .resolve_type_handle(&PREALLOCATED_HANDLE_TYPE_UNARY_PREDICATE)
            .is_some());
        assert!(state
            .resolve_type_handle(&PREALLOCATED_HANDLE_TYPE_BINARY_CONNECTIVE)
            .is_some());
        assert!(state
            .resolve_type_handle(&PREALLOCATED_HANDLE_TYPE_UNARY_CONNECTIVE)
            .is_some());
        assert!(state
            .resolve_type_handle(&PREALLOCATED_HANDLE_TYPE_QUANTIFIER)
            .is_some());
        assert!(state
            .resolve_type_handle(&PREALLOCATED_HANDLE_TYPE_BETA)
            .is_some());
        assert!(state
            .resolve_type_handle(&PREALLOCATED_HANDLE_TYPE_ALPHA)
            .is_some());
    }

    /// Tests all primitive terms are registered in the initial theory.
    #[test]
    pub fn initial_theory3() {
        let state = RuntimeState::new();

        assert!(state
            .resolve_term_handle(&PREALLOCATED_HANDLE_TERM_EXISTS)
            .is_ok());
        assert!(state
            .resolve_term_handle(&PREALLOCATED_HANDLE_TERM_FORALL)
            .is_ok());
        assert!(state
            .resolve_term_handle(&PREALLOCATED_HANDLE_TERM_IMPLICATION)
            .is_ok());
        assert!(state
            .resolve_term_handle(&PREALLOCATED_HANDLE_TERM_CONJUNCTION)
            .is_ok());
        assert!(state
            .resolve_term_handle(&PREALLOCATED_HANDLE_TERM_DISJUNCTION)
            .is_ok());
        assert!(state
            .resolve_term_handle(&PREALLOCATED_HANDLE_TERM_TRUE)
            .is_ok());
        assert!(state
            .resolve_term_handle(&PREALLOCATED_HANDLE_TERM_FALSE)
            .is_ok());
        assert!(state
            .resolve_term_handle(&PREALLOCATED_HANDLE_TERM_EQUALITY)
            .is_ok());
        assert!(state
            .resolve_term_handle(&PREALLOCATED_HANDLE_TERM_NEGATION)
            .is_ok());
    }

    ////////////////////////////////////////////////////////////////////////////
    // Free-variable tests.
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    pub fn free_variables0() {
        let mut state = RuntimeState::new();

        let v = state
            .term_register_variable("a", PREALLOCATED_HANDLE_TYPE_PROP)
            .unwrap();

        let fvs = state.term_free_variables(&v).unwrap();

        assert_eq!(
            fvs,
            vec![(&String::from("a"), &PREALLOCATED_HANDLE_TYPE_PROP)]
        );
    }

    #[test]
    pub fn free_variables1() {
        let state = RuntimeState::new();

        let fvs = state
            .term_free_variables(&PREALLOCATED_HANDLE_TERM_TRUE)
            .unwrap();

        assert!(fvs.is_empty());
    }

    #[test]
    pub fn free_variables2() {
        let mut state = RuntimeState::new();

        let v = state
            .term_register_variable("a", PREALLOCATED_HANDLE_TYPE_PROP)
            .unwrap();
        let l = state
            .term_register_lambda("a", PREALLOCATED_HANDLE_TYPE_PROP, v)
            .unwrap();

        let fvs = state.term_free_variables(&l).unwrap();

        assert!(fvs.is_empty());
    }

    #[test]
    pub fn free_variables3() {
        let mut state = RuntimeState::new();

        let v = state
            .term_register_variable("a", PREALLOCATED_HANDLE_TYPE_BINARY_PREDICATE)
            .unwrap();
        let l = state
            .term_register_lambda("a", PREALLOCATED_HANDLE_TYPE_PROP, v)
            .unwrap();

        let fvs = state.term_free_variables(&l).unwrap();

        assert_eq!(
            fvs,
            vec![(
                &String::from("a"),
                &PREALLOCATED_HANDLE_TYPE_BINARY_PREDICATE
            )]
        );
    }

    #[test]
    pub fn free_variables4() {
        let mut state = RuntimeState::new();

        let l = state
            .term_register_lambda(
                "a",
                PREALLOCATED_HANDLE_TYPE_PROP,
                PREALLOCATED_HANDLE_TERM_TRUE,
            )
            .unwrap();

        let fvs = state.term_free_variables(&l).unwrap();

        assert!(fvs.is_empty())
    }

    #[test]
    pub fn free_variables5() {
        let mut state = RuntimeState::new();

        let l = state
            .term_register_lambda(
                "a",
                PREALLOCATED_HANDLE_TYPE_PROP,
                PREALLOCATED_HANDLE_TERM_TRUE,
            )
            .unwrap();
        let v = state
            .term_register_variable("v", PREALLOCATED_HANDLE_TYPE_PROP)
            .unwrap();
        let t = state.term_register_application(l, v).unwrap();

        let fvs = state.term_free_variables(&t).unwrap();

        assert_eq!(
            fvs,
            vec![(&String::from("v"), &PREALLOCATED_HANDLE_TYPE_PROP)]
        )
    }

    ////////////////////////////////////////////////////////////////////////////
    // Alpha-equivalence tests.
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    pub fn alpha_equivalence0() {
        let mut state = RuntimeState::new();

        let v = state
            .term_register_variable("a", PREALLOCATED_HANDLE_TYPE_PROP)
            .unwrap();

        assert!(state.is_alpha_equivalent(&v, &v).unwrap());
    }

    #[test]
    pub fn alpha_equivalence1() {
        let mut state = RuntimeState::new();

        let v = state
            .term_register_variable("a", PREALLOCATED_HANDLE_TYPE_PROP)
            .unwrap();
        let q = state
            .term_register_variable("b", PREALLOCATED_HANDLE_TYPE_PROP)
            .unwrap();

        assert!(!state.is_alpha_equivalent(&v, &q).unwrap());
    }

    #[test]
    pub fn alpha_equivalence2() {
        let mut state = RuntimeState::new();

        let v = state
            .term_register_variable("a", PREALLOCATED_HANDLE_TYPE_PROP)
            .unwrap();
        let q = state
            .term_register_variable("a", PREALLOCATED_HANDLE_TYPE_BINARY_PREDICATE)
            .unwrap();

        assert!(!state.is_alpha_equivalent(&v, &q).unwrap());
    }

    #[test]
    pub fn alpha_equivalence3() {
        let mut state = RuntimeState::new();

        let v = state
            .term_register_variable("a", PREALLOCATED_HANDLE_TYPE_PROP)
            .unwrap();
        let l = state
            .term_register_lambda("a", PREALLOCATED_HANDLE_TYPE_PROP, v.clone())
            .unwrap();
        let c = state.term_register_application(l, v).unwrap();

        assert!(state.is_alpha_equivalent(&c, &c).unwrap());
    }

    #[test]
    pub fn alpha_equivalence4() {
        let mut state = RuntimeState::new();

        let v0 = state
            .term_register_variable("a", PREALLOCATED_HANDLE_TYPE_PROP)
            .unwrap();
        let l0 = state
            .term_register_lambda("a", PREALLOCATED_HANDLE_TYPE_PROP, v0.clone())
            .unwrap();
        let c0 = state.term_register_application(l0, v0).unwrap();

        let v1 = state
            .term_register_variable("b", PREALLOCATED_HANDLE_TYPE_PROP)
            .unwrap();
        let l1 = state
            .term_register_lambda("b", PREALLOCATED_HANDLE_TYPE_PROP, v1.clone())
            .unwrap();
        let c1 = state.term_register_application(l1, v1).unwrap();

        assert!(state.is_alpha_equivalent(&c0, &c1).unwrap());
    }

    ////////////////////////////////////////////////////////////////////////////
    // Substitution tests.
    ////////////////////////////////////////////////////////////////////////////

    ////////////////////////////////////////////////////////////////////////////
    // Type-checking tests.
    ////////////////////////////////////////////////////////////////////////////

    ////////////////////////////////////////////////////////////////////////////
    // Inference tests.
    ////////////////////////////////////////////////////////////////////////////
}
