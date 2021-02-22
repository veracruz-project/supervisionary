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

use std::{collections::HashMap, iter::FromIterator};

use crate::kernel::{
    _type::{
        Type, TYPE_ALPHA, TYPE_BETA, TYPE_BINARY_CONNECTIVE, TYPE_POLYMORPHIC_BINARY_PREDICATE,
        TYPE_POLYMORPHIC_QUANTIFIER, TYPE_POLYMORPHIC_UNARY_PREDICATE, TYPE_PROP,
        TYPE_UNARY_CONNECTIVE,
    },
    error_code::ErrorCode,
    handle::{
        Handle, PREALLOCATED_HANDLE_CONSTANT_CONJUNCTION, PREALLOCATED_HANDLE_CONSTANT_DISJUNCTION,
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
        PREALLOCATED_HANDLE_UPPER_BOUND,
    },
    kernel_panic::{DANGLING_HANDLE_ERROR, HANDLE_EXHAUST_ERROR, PRIMITIVE_CONSTRUCTION_ERROR},
    name::Name,
    term::{
        Term, TERM_CONJUNCTION_CONSTANT, TERM_DISJUNCTION_CONSTANT, TERM_EQUALITY_CONSTANT,
        TERM_EXISTS_CONSTANT, TERM_FALSE_CONSTANT, TERM_FORALL_CONSTANT, TERM_IMPLICATION_CONSTANT,
        TERM_NEGATION_CONSTANT, TERM_TRUE_CONSTANT,
    },
    theorem::Theorem,
};

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
    next_handle: Handle,
    /// The table of registered type-formers.  Handles are essentially names for
    /// type-formers.
    type_formers: HashMap<Handle, usize>,
    /// The table of types.  The kernel enforces maximal sharing, wherein any
    /// attempt to register a previously-registered type means that the handle
    /// pointing to the registered type is returned.
    types: HashMap<Handle, Type>,
    /// The table of constants, associating handles for constants to handles for
    /// types.  Handles are essentially names for constants.
    constants: HashMap<Handle, Handle>,
    /// The table of terms.  The kernel enforces maximal sharing, wherein any
    /// attempt to register a previously-registered term (up-to
    /// alpha-equivalence) means that the handle pointing to the registered term
    /// is returned.
    terms: HashMap<Handle, Term>,
    /// The table of theorems.  The kernel enforces maximal sharing, wherein any
    /// attempt to register a previously-registered theorem (up-to
    /// alpha-equivalence of the conclusion and hypotheses) means that the
    /// handle pointing to the registered theorem is returned.
    theorems: HashMap<Handle, Theorem>,
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
    fn issue_handle(&mut self) -> Handle {
        let next = self.next_handle;

        match self.next_handle.checked_add(1) {
            None => panic!(HANDLE_EXHAUST_ERROR),
            Some(next) => self.next_handle = next,
        }

        return next;
    }

    ////////////////////////////////////////////////////////////////////////////
    // Type-former related material.
    ////////////////////////////////////////////////////////////////////////////

    /// Registers a new type-former with a declared arity with the runtime
    /// state.  Returns the handle to the newly-registered type-former.
    pub fn type_former_handle_register<T>(&mut self, arity: T) -> Handle
    where
        T: Into<usize>,
    {
        let handle = self.issue_handle();
        self.type_formers.insert(handle, arity.into());
        handle
    }

    /// Returns Some(`arity`) if the type-former pointed-to by `handle` has
    /// arity `arity`.
    #[inline]
    pub fn type_former_handle_resolve(&self, handle: &Handle) -> Option<&usize> {
        self.type_formers.get(handle)
    }

    /// Returns `true` iff `handle` points to a type-former registered with the
    /// runtime state.
    #[inline]
    pub fn type_former_handle_is_registered(&self, handle: &Handle) -> bool {
        self.type_former_handle_resolve(handle).is_some()
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
    fn admit_type(&mut self, tau: Type) -> Handle {
        for (handle, registered) in self.types.iter() {
            if registered == &tau {
                return *handle;
            }
        }

        let handle = self.issue_handle();
        self.types.insert(handle, tau);
        handle
    }

    /// Returns `Some(tau)` iff the handle points to a type, `tau` in the
    /// runtime state's type-table.
    #[inline]
    pub fn resolve_type_handle(&self, handle: &Handle) -> Option<&Type> {
        self.types.get(handle)
    }

    /// Returns `true` iff the handle points to a type, `tau`, in the runtime
    /// state's type-table.
    #[inline]
    pub fn type_handle_is_registered(&self, handle: &Handle) -> bool {
        self.resolve_type_handle(handle).is_some()
    }

    /// Registers a new type in the runtime state's type-table with a given
    /// name.  Returns the handle of the newly-allocated type (or the existing
    /// handle, if the type-variable already appears in the type-table).
    #[inline]
    pub fn type_handle_register_variable<T>(&mut self, name: T) -> Handle
    where
        T: Into<Name>,
    {
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
    pub fn type_handle_register_combination(
        &mut self,
        former: Handle,
        arguments: Vec<Handle>,
    ) -> Result<Handle, ErrorCode> {
        let arity = self
            .type_former_handle_resolve(&former)
            .ok_or(ErrorCode::NoSuchTypeFormerRegistered)?;

        if !arguments.iter().all(|a| self.type_handle_is_registered(a)) {
            return Err(ErrorCode::NoSuchTypeRegistered);
        }

        if arguments.len() != *arity {
            return Err(ErrorCode::MismatchedArity);
        }

        Ok(self.admit_type(Type::combination(former, arguments.iter().cloned())))
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
    pub fn type_handle_register_function(
        &mut self,
        domain: Handle,
        range: Handle,
    ) -> Result<Handle, ErrorCode> {
        if !self.type_handle_is_registered(&domain) || !self.type_handle_is_registered(&range) {
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
    pub fn type_split_variable(&self, handle: &Handle) -> Result<&Name, ErrorCode> {
        if let Some(tau) = self.resolve_type_handle(handle) {
            tau.split_variable().ok_or(ErrorCode::NotATypeVariable)
        } else {
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
    pub fn type_split_combination(
        &self,
        handle: &Handle,
    ) -> Result<(&Handle, &Vec<Handle>), ErrorCode> {
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
    pub fn type_split_function(&self, handle: &Handle) -> Result<(&Handle, &Handle), ErrorCode> {
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
    pub fn type_test_is_variable(&self, handle: &Handle) -> Result<bool, ErrorCode> {
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
    pub fn type_test_is_combination(&self, handle: &Handle) -> Result<bool, ErrorCode> {
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
    pub fn type_test_is_function(&self, handle: &Handle) -> Result<bool, ErrorCode> {
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
    pub fn type_ftv(&self, handle: &Handle) -> Result<Vec<&Name>, ErrorCode> {
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
    pub fn type_substitute<T>(&mut self, tau: &Handle, sigma: T) -> Result<Handle, ErrorCode>
    where
        T: Iterator<Item = (Name, Handle)> + Clone,
    {
        let mut tau = self
            .resolve_type_handle(tau)
            .ok_or(ErrorCode::NoSuchTypeRegistered)?
            .clone();

        for (domain, range) in sigma.clone() {
            let range = self
                .resolve_type_handle(&range)
                .ok_or(ErrorCode::NoSuchTypeRegistered)?;

            match tau {
                Type::Variable { ref name } => {
                    if name == &domain {
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
    pub fn constant_handle_register(&mut self, handle: Handle) -> Result<Handle, ErrorCode> {
        if !self.type_handle_is_registered(&handle) {
            return Err(ErrorCode::NoSuchTypeRegistered);
        }

        let fresh = self.issue_handle();
        self.constants.insert(fresh, handle);
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
    pub fn constant_handle_resolve(&self, handle: &Handle) -> Result<&Handle, ErrorCode> {
        self.constants
            .get(handle)
            .ok_or(ErrorCode::NoSuchConstantRegistered)
    }

    /// Returns `true` iff `handle` points-to a registered constant in the
    /// runtime state's constant table.
    #[inline]
    pub fn constant_handle_is_registered(&self, handle: &Handle) -> bool {
        self.constant_handle_resolve(handle).is_some()
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
    fn admit_term(&mut self, trm: Term) -> Handle {
        for (handle, registered) in self.terms.clone().iter() {
            if self
                .is_alpha_equivalent_inner(&trm, &registered)
                .expect(DANGLING_HANDLE_ERROR)
            {
                return *handle;
            }
        }

        let fresh = self.issue_handle();
        self.terms.insert(fresh, trm);
        fresh
    }

    /// Registers a new term variable, with name `name` and with the type
    /// pointed-to by handle in the runtime state's type-table.
    ///
    /// # Errors
    ///
    /// Returns `Err(ErrorCode::NoSuchTypeRegistered)` if `handle` does not
    /// point to any type in the runtime state's type-table.
    pub fn register_variable<T>(&mut self, name: T, handle: Handle) -> Result<Handle, ErrorCode>
    where
        T: Into<Name>,
    {
        if !self.type_handle_is_registered(&handle) {
            return Err(ErrorCode::NoSuchTypeRegistered);
        }

        Ok(self.admit_term(Term::variable(name, handle)))
    }

    /// Registers a new term constant, lifting the handle pointing-to a
    /// registered constant in the runtime state's constant-table into a term.
    /// Uses the constant's registered type as the type of the lifted constant.
    ///
    /// # Errors
    ///
    /// Returns `Err(ErrorCode::NoSuchConstantRegistered)` if `handle` does not
    /// point-to a registered constant in the runtime state's constant-table.
    pub fn register_constant_at_default_type(
        &mut self,
        handle: Handle,
    ) -> Result<Handle, ErrorCode> {
        let tau = self.constant_handle_resolve(&handle)?;

        let tau = tau.clone();

        Ok(self.admit_term(Term::constant(handle, tau)))
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
    pub fn register_constant_at_constrained_type<T>(
        &mut self,
        handle: Handle,
        type_substitution: T,
    ) -> Result<Handle, ErrorCode>
    where
        T: Iterator<Item = (Name, Handle)> + Clone,
    {
        let tau = self.clone().constant_handle_resolve(&handle)?;
        let tau = self.type_substitute(tau, type_substitution)?;
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
    pub fn register_application(
        &mut self,
        left: Handle,
        right: Handle,
    ) -> Result<Handle, ErrorCode> {
        if !self.is_term_registered(&left) {
            return Err(ErrorCode::NoSuchTermRegistered);
        }

        if !self.is_term_registered(&right) {
            return Err(ErrorCode::NoSuchTermRegistered);
        }

        let ltau = self.infer_type(&left)?;
        let rtau = self.infer_type(&right)?;

        let (dom, _rng) = self.type_split_function(&ltau)?;

        if dom != &rtau {
            return Err(ErrorCode::DomainTypeMismatch);
        }

        Ok(self.admit_term(Term::application(left, right)))
    }

    /// Registers a new lambda-abstraction into the runtime state's term-table
    /// with a name, a type pointed-to by the handle `_type`, and a body term
    /// pointed-to by the handle `handle`.
    ///
    /// # Errors
    ///
    /// Returns `Err(ErrorCode::NoSuchTypeRegistered)` if `_type` does not
    /// point-to a registered type in the runtime state's type-table.
    ///
    /// Returns `Err(ErrorCode::NoSuchTermRegistered)`  if `handle` does not
    /// point-to a registered term in the runtime state's term-table.
    pub fn register_lambda<T>(
        &mut self,
        name: T,
        _type: Handle,
        body: Handle,
    ) -> Result<Handle, ErrorCode>
    where
        T: Into<Name>,
    {
        if !self.type_handle_is_registered(&_type) {
            return Err(ErrorCode::NoSuchTypeRegistered);
        }

        if !self.is_term_registered(&body) {
            return Err(ErrorCode::NoSuchTermRegistered);
        }

        Ok(self.admit_term(Term::lambda(name, _type, body)))
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
    pub fn register_negation(&mut self, term: Handle) -> Result<Handle, ErrorCode> {
        if !self.is_proposition(&term)? {
            return Err(ErrorCode::NotAProposition);
        }

        self.register_application(PREALLOCATED_HANDLE_TERM_NEGATION, term)
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
    pub fn register_equality(&mut self, left: Handle, right: Handle) -> Result<Handle, ErrorCode> {
        let ltau = self.infer_type(&left)?;
        let rtau = self.infer_type(&right)?;

        if ltau != rtau {
            return Err(ErrorCode::DomainTypeMismatch);
        }

        let sigma = vec![(String::from("A"), ltau)];
        let spec = self.instantiation(&PREALLOCATED_HANDLE_TERM_EQUALITY, sigma.iter().cloned())?;
        let inner = self.register_application(spec, left)?;
        self.register_application(inner, right)
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
    pub fn register_disjunction(
        &mut self,
        left: Handle,
        right: Handle,
    ) -> Result<Handle, ErrorCode> {
        if !self.is_proposition(&left)? {
            return Err(ErrorCode::NotAProposition);
        }

        if !self.is_proposition(&right)? {
            return Err(ErrorCode::NotAProposition);
        }

        let inner = self.register_application(PREALLOCATED_HANDLE_TERM_DISJUNCTION, left)?;
        self.register_application(inner, right)
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
    pub fn register_conjunction(
        &mut self,
        left: Handle,
        right: Handle,
    ) -> Result<Handle, ErrorCode> {
        if !self.is_proposition(&left)? {
            return Err(ErrorCode::NotAProposition);
        }

        if !self.is_proposition(&right)? {
            return Err(ErrorCode::NotAProposition);
        }

        let inner = self.register_application(PREALLOCATED_HANDLE_TERM_CONJUNCTION, left)?;
        self.register_application(inner, right)
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
    pub fn register_implication(
        &mut self,
        left: Handle,
        right: Handle,
    ) -> Result<Handle, ErrorCode> {
        if !self.is_proposition(&left)? {
            return Err(ErrorCode::NotAProposition);
        }

        if !self.is_proposition(&right)? {
            return Err(ErrorCode::NotAProposition);
        }

        let inner = self.register_application(PREALLOCATED_HANDLE_TERM_IMPLICATION, left)?;
        self.register_application(inner, right)
    }

    /// Registers a new universal quantifier from a type, pointed-to by `_type`,
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
    /// Returns `Err(ErrorCode::NoSuchTypeRegistered)` if `_type` does not
    /// point-to any registered type in the runtime state's type-table.
    pub fn register_forall<T>(
        &mut self,
        name: T,
        _type: Handle,
        body: Handle,
    ) -> Result<Handle, ErrorCode>
    where
        T: Into<Name>,
    {
        if !self.type_handle_is_registered(&_type) {
            return Err(ErrorCode::NoSuchTypeRegistered);
        }

        if !self.is_proposition(&body)? {
            return Err(ErrorCode::NotAProposition);
        }

        let sigma = vec![(String::from("A"), _type.clone())];

        let lambda = self
            .register_lambda(name, _type, body)
            .expect(DANGLING_HANDLE_ERROR);

        let univ = self
            .instantiation(&PREALLOCATED_HANDLE_TERM_FORALL, sigma.iter().cloned())
            .expect(DANGLING_HANDLE_ERROR);

        self.register_application(univ, lambda)
    }

    /// Registers a new existential quantifier from a type, pointed-to by
    /// `_type`, a body, pointed to by `body`, and a name.
    ///
    /// # Errors
    ///
    /// Returns `Err(ErrorCode::NoSuchTermRegistered)` if `body` does not
    /// point-to any registered term in the runtime state's term-table.
    ///
    /// Returns `Err(ErrorCode::NotAProposition)` if the term pointed-to by
    /// `body` is not a proposition.
    ///
    /// Returns `Err(ErrorCode::NoSuchTypeRegistered)` if `_type` does not
    /// point-to any registered type in the runtime state's type-table.
    pub fn register_exists<T>(
        &mut self,
        name: T,
        _type: Handle,
        body: Handle,
    ) -> Result<Handle, ErrorCode>
    where
        T: Into<Name>,
    {
        if !self.type_handle_is_registered(&_type) {
            return Err(ErrorCode::NoSuchTypeRegistered);
        }

        if !self.is_proposition(&body)? {
            return Err(ErrorCode::NotAProposition);
        }

        let sigma = vec![(String::from("A"), _type.clone())];

        let lambda = self
            .register_lambda(name, _type, body)
            .expect(DANGLING_HANDLE_ERROR);

        let univ = self
            .instantiation(&PREALLOCATED_HANDLE_TERM_EXISTS, sigma.iter().cloned())
            .expect(DANGLING_HANDLE_ERROR);

        self.register_application(univ, lambda)
    }

    /// Returns `Ok(trm)` iff `handle` points-to the term `trm` in the runtime
    /// state's term-table.
    ///
    /// # Errors
    ///
    /// Returns `Err(ErrorCode::NoSuchTermRegistered)` if `handle` does not
    /// point-to any registered term in the runtime state's term-table.
    #[inline]
    pub fn resolve_term_handle(&self, handle: &Handle) -> Result<&Term, ErrorCode> {
        self.terms
            .get(handle)
            .ok_or(ErrorCode::NoSuchTermRegistered)
    }

    /// Returns `true` iff `handle` points-to a registered term in the runtime
    /// state's term-table.
    #[inline]
    pub fn is_term_registered(&self, handle: &Handle) -> bool {
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
    pub fn split_variable(&self, handle: &Handle) -> Result<(&Name, &Handle), ErrorCode> {
        let trm = self.resolve_term_handle(handle)?;

        if let Term::Variable { name, _type } = trm {
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
    pub fn split_constant(&self, handle: &Handle) -> Result<(&Handle, &Handle), ErrorCode> {
        let trm = self.resolve_term_handle(handle)?;

        if let Term::Constant { handle, _type } = trm {
            Ok((handle, _type))
        } else {
            Err(ErrorCode::NotAConstant)
        }
    }

    pub fn split_application(&self, handle: &Handle) -> Result<(&Handle, &Handle), ErrorCode> {
        let trm = self.resolve_term_handle(handle)?;

        if let Term::Application { left, right } = trm {
            Ok((left, right))
        } else {
            Err(ErrorCode::NotAnApplication)
        }
    }

    pub fn split_lambda(&self, handle: &Handle) -> Result<(&Name, &Handle, &Handle), ErrorCode> {
        let trm = self.resolve_term_handle(handle)?;

        if let Term::Lambda { name, _type, body } = trm {
            Ok((name, _type, body))
        } else {
            Err(ErrorCode::NotALambda)
        }
    }

    pub fn split_negation(&self, handle: &Handle) -> Result<&Handle, ErrorCode> {
        let (left, right) = self
            .resolve_term_handle(handle)
            .ok_or(ErrorCode::NoSuchTermRegistered)?
            .split_application()
            .ok_or(ErrorCode::NotANegation)?;

        let (constant, _tau) = self
            .split_constant(left)
            .map_err(|_e| ErrorCode::NotANegation)?;

        if constant != &PREALLOCATED_HANDLE_CONSTANT_NEGATION {
            return Err(ErrorCode::NotANegation);
        }

        Ok(right)
    }

    pub fn split_equality(&self, handle: &Handle) -> Result<(&Handle, &Handle), ErrorCode> {
        let (left, right) = self
            .resolve_term_handle(handle)
            .ok_or(ErrorCode::NoSuchTermRegistered)?
            .split_application()
            .ok_or(ErrorCode::NotAnEquality)?;

        let (left, mid) = self
            .resolve_term_handle(left)
            .expect(DANGLING_HANDLE_ERROR)
            .split_application()
            .ok_or(ErrorCode::NotAnEquality)?;

        let (constant, _tau) = self
            .split_constant(left)
            .map_err(|_e| ErrorCode::NotAnEquality)?;

        if constant == &PREALLOCATED_HANDLE_CONSTANT_EQUALITY {
            Ok((mid, right))
        } else {
            Err(ErrorCode::NotAnEquality)
        }
    }

    pub fn split_disjunction(&self, handle: &Handle) -> Result<(&Handle, &Handle), ErrorCode> {
        let (left, right) = self
            .resolve_term_handle(handle)
            .ok_or(ErrorCode::NoSuchTermRegistered)?
            .split_application()
            .ok_or(ErrorCode::NotADisjunction)?;

        let (left, mid) = self
            .resolve_term_handle(left)
            .expect(DANGLING_HANDLE_ERROR)
            .split_application()
            .ok_or(ErrorCode::NotADisjunction)?;

        let (constant, _tau) = self
            .split_constant(left)
            .map_err(|_e| ErrorCode::NotADisjunction)?;

        if constant == &PREALLOCATED_HANDLE_CONSTANT_DISJUNCTION {
            Ok((mid, right))
        } else {
            Err(ErrorCode::NotADisjunction)
        }
    }

    pub fn split_conjunction(&self, handle: &Handle) -> Result<(&Handle, &Handle), ErrorCode> {
        let (left, right) = self
            .resolve_term_handle(handle)
            .ok_or(ErrorCode::NoSuchTermRegistered)?
            .split_application()
            .ok_or(ErrorCode::NotAConjunction)?;

        let (left, mid) = self
            .resolve_term_handle(left)
            .expect(DANGLING_HANDLE_ERROR)
            .split_application()
            .ok_or(ErrorCode::NotAConjunction)?;

        let (constant, _tau) = self
            .split_constant(left)
            .map_err(|_e| ErrorCode::NotAConjunction)?;

        if constant == &PREALLOCATED_HANDLE_CONSTANT_CONJUNCTION {
            Ok((mid, right))
        } else {
            Err(ErrorCode::NotAConjunction)
        }
    }

    pub fn split_implication(&self, handle: &Handle) -> Result<(&Handle, &Handle), ErrorCode> {
        let (left, right) = self
            .resolve_term_handle(handle)
            .ok_or(ErrorCode::NoSuchTermRegistered)?
            .split_application()
            .ok_or(ErrorCode::NotAnImplication)?;

        let (left, mid) = self
            .resolve_term_handle(left)
            .expect(DANGLING_HANDLE_ERROR)
            .split_application()
            .ok_or(ErrorCode::NotAnImplication)?;

        let (constant, _tau) = self
            .split_constant(left)
            .map_err(|_e| ErrorCode::NotAnImplication)?;

        if constant == &PREALLOCATED_HANDLE_CONSTANT_IMPLICATION {
            Ok((mid, right))
        } else {
            Err(ErrorCode::NotAnImplication)
        }
    }

    pub fn split_forall(&self, handle: &Handle) -> Result<(&Name, &Handle, &Handle), ErrorCode> {
        let (left, right) = self
            .resolve_term_handle(handle)
            .ok_or(ErrorCode::NoSuchTermRegistered)?
            .split_application()
            .ok_or(ErrorCode::NotAForall)?;

        let (constant, _tau) = self
            .split_constant(left)
            .map_err(|_e| ErrorCode::NotAForall)?;

        let (name, _type, body) = self
            .split_lambda(right)
            .map_err(|_e| ErrorCode::NotAForall)?;

        if constant == &PREALLOCATED_HANDLE_CONSTANT_FORALL {
            Ok((name, _type, body))
        } else {
            Err(ErrorCode::NotAForall)
        }
    }

    pub fn split_exists(&self, handle: &Handle) -> Result<(&Name, &Handle, &Handle), ErrorCode> {
        let (left, right) = self
            .resolve_term_handle(handle)
            .ok_or(ErrorCode::NoSuchTermRegistered)?
            .split_application()
            .ok_or(ErrorCode::NotAnExists)?;

        let (constant, _tau) = self
            .split_constant(left)
            .map_err(|_e| ErrorCode::NotAnExists)?;

        let (name, _type, body) = self
            .split_lambda(right)
            .map_err(|_e| ErrorCode::NotAnExists)?;

        if constant == &PREALLOCATED_HANDLE_CONSTANT_EXISTS {
            Ok((name, _type, body))
        } else {
            Err(ErrorCode::NotAnExists)
        }
    }

    #[inline]
    pub fn is_variable(&self, handle: &Handle) -> Result<bool, ErrorCode> {
        Ok(self.split_variable(handle).is_ok())
    }

    #[inline]
    pub fn is_constant(&self, handle: &Handle) -> Result<bool, ErrorCode> {
        Ok(self.split_constant(handle).is_ok())
    }

    #[inline]
    pub fn is_application(&self, handle: &Handle) -> Result<bool, ErrorCode> {
        Ok(self.split_application(handle).is_ok())
    }

    #[inline]
    pub fn is_lambda(&self, handle: &Handle) -> Result<bool, ErrorCode> {
        Ok(self.split_lambda(handle).is_ok())
    }

    pub fn is_true(&self, handle: &Handle) -> Result<bool, ErrorCode> {
        let (handle, tau) = self
            .resolve_term_handle(handle)
            .ok_or(ErrorCode::NoSuchTermRegistered)?
            .split_constant()
            .ok_or(ErrorCode::NotAConstant)?;

        Ok(handle == &PREALLOCATED_HANDLE_CONSTANT_TRUE && tau == &PREALLOCATED_HANDLE_TYPE_PROP)
    }

    pub fn is_false(&self, handle: &Handle) -> Result<bool, ErrorCode> {
        let (handle, tau) = self
            .resolve_term_handle(handle)
            .ok_or(ErrorCode::NoSuchTermRegistered)?
            .split_constant()
            .ok_or(ErrorCode::NotAConstant)?;

        Ok(handle == &PREALLOCATED_HANDLE_CONSTANT_FALSE && tau == &PREALLOCATED_HANDLE_TYPE_PROP)
    }

    #[inline]
    pub fn is_negation(&self, handle: &Handle) -> Result<bool, ErrorCode> {
        Ok(self.split_negation(handle).is_ok())
    }

    #[inline]
    pub fn is_equality(&self, handle: &Handle) -> Result<bool, ErrorCode> {
        Ok(self.split_equality(handle).is_ok())
    }

    #[inline]
    pub fn is_disjunction(&self, handle: &Handle) -> Result<bool, ErrorCode> {
        Ok(self.split_disjunction(handle).is_ok())
    }

    #[inline]
    pub fn is_conjunction(&self, handle: &Handle) -> Result<bool, ErrorCode> {
        Ok(self.split_conjunction(handle).is_ok())
    }

    #[inline]
    pub fn is_implication(&self, handle: &Handle) -> Result<bool, ErrorCode> {
        Ok(self.split_implication(handle).is_ok())
    }

    #[inline]
    pub fn is_forall(&self, handle: &Handle) -> Result<bool, ErrorCode> {
        Ok(self.split_forall(handle).is_ok())
    }

    #[inline]
    pub fn is_exists(&self, handle: &Handle) -> Result<bool, ErrorCode> {
        Ok(self.split_exists(handle).is_ok())
    }

    /// Computes the *free type-variables* of the term pointed-to by the handle
    /// `handle` in the runtime state's term-table.
    ///
    /// # Errors
    ///
    /// Returns `Err(ErrorCode::NoSuchTermRegistered)` if `handle` does not
    /// point-to any term in the runtime state's term-table.
    pub fn term_ftv(&self, handle: &Handle) -> Result<Vec<&Name>, ErrorCode> {
        let trm = self.resolve_term_handle(handle)?;

        let mut work_list = vec![trm];
        let mut ftv = vec![];

        while let Some(next) = work_list.pop() {
            match next {
                Term::Variable { _type, .. } => {
                    let mut fvs = self.type_ftv(_type).expect(DANGLING_HANDLE_ERROR);
                    ftv.append(&mut fvs);
                }
                Term::Constant { _type, .. } => {
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
                Term::Lambda { _type, body, .. } => {
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

    pub fn term_fv(&self, handle: &Handle) -> Result<Vec<(&Name, &Handle)>, ErrorCode> {
        let term = self.resolve_term_handle(handle)?;

        match term {
            Term::Variable { name, _type } => Ok(vec![(name, _type)]),
            Term::Constant { .. } => Ok(vec![]),
            Term::Application { left, right } => {
                let mut left = self.term_fv(left).expect(DANGLING_HANDLE_ERROR);
                let mut right = self.term_fv(right).expect(DANGLING_HANDLE_ERROR);

                left.append(&mut right);

                Ok(left)
            }
            Term::Lambda { name, _type, body } => {
                let body = self.term_fv(body).expect(DANGLING_HANDLE_ERROR);

                Ok(body
                    .iter()
                    .filter(|v| **v != (name, _type))
                    .cloned()
                    .collect())
            }
        }
    }

    pub fn substitution<T>(&mut self, handle: &Handle, sigma: T) -> Result<Handle, ErrorCode>
    where
        T: Iterator<Item = (Name, Handle)>,
    {
        unimplemented!()
    }

    pub fn instantiation<T>(&mut self, handle: &Handle, sigma: T) -> Result<Handle, ErrorCode>
    where
        T: Iterator<Item = (Name, Handle)>,
    {
        unimplemented!()
    }

    pub fn infer_type(&mut self, handle: &Handle) -> Result<Handle, ErrorCode> {
        let trm = self
            .resolve_term_handle(handle)
            .ok_or(ErrorCode::NoSuchTermRegistered)?;

        let trm = trm.clone();

        match trm {
            Term::Variable { _type, .. } => Ok(_type.clone()),
            Term::Constant { _type, .. } => Ok(_type.clone()),
            Term::Application { left, right } => {
                let ltau = self.infer_type(&left)?;
                let rtau = self.infer_type(&right)?;

                let (dom, rng) = self
                    .type_split_function(&ltau)
                    .map_err(|_e| ErrorCode::NotAFunctionType)?;

                if dom == &rtau {
                    Ok(rng.clone())
                } else {
                    Err(ErrorCode::DomainTypeMismatch)
                }
            }
            Term::Lambda { _type, body, .. } => {
                let btau = self.infer_type(&body)?;
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
    pub fn is_proposition(&mut self, handle: &Handle) -> Result<bool, ErrorCode> {
        Ok(self.infer_type(handle)? == PREALLOCATED_HANDLE_TYPE_PROP)
    }

    fn swap(&mut self, handle: &Handle, a: Name, b: Name) -> Result<Handle, ErrorCode> {
        let trm = self
            .resolve_term_handle(handle)
            .ok_or(ErrorCode::NoSuchTermRegistered)?
            .clone();

        match trm {
            Term::Variable { name, _type } => {
                if name == a {
                    Ok(self.admit_term(Term::variable(b, _type.clone())))
                } else if name == b {
                    Ok(self.admit_term(Term::variable(a, _type.clone())))
                } else {
                    Ok(handle.clone())
                }
            }
            Term::Constant { .. } => Ok(handle.clone()),
            Term::Application { left, right } => {
                let left = self
                    .swap(&left, a.clone(), b.clone())
                    .expect(DANGLING_HANDLE_ERROR);
                let right = self.swap(&right, a, b).expect(DANGLING_HANDLE_ERROR);

                Ok(self.admit_term(Term::application(left, right)))
            }
            Term::Lambda { name, _type, body } => {
                let body = self
                    .swap(&body, a.clone(), b.clone())
                    .expect(DANGLING_HANDLE_ERROR);
                if name == a {
                    Ok(self.admit_term(Term::lambda(b, _type.clone(), body)))
                } else if name == b {
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
                    _type: _type0,
                },
                Term::Variable {
                    name: name1,
                    _type: _type1,
                },
            ) => Ok(name0 == name1 && _type0 == _type1),
            (
                Term::Constant {
                    handle: handle0,
                    _type: _type0,
                },
                Term::Constant {
                    handle: handle1,
                    _type: _type1,
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
                    _type: _type0,
                    body: body0,
                },
                Term::Lambda {
                    name: name1,
                    _type: _type1,
                    body: body1,
                },
            ) => {
                if name0 == name1 {
                    let body = self.is_alpha_equivalent(body0, body1)?;
                    Ok(body && _type0 == _type1)
                } else if !self
                    .term_fv(body1)
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

    pub fn is_alpha_equivalent(
        &mut self,
        left: &Handle,
        right: &Handle,
    ) -> Result<bool, ErrorCode> {
        let left = self
            .resolve_term_handle(left)
            .ok_or(ErrorCode::NoSuchTermRegistered)?
            .clone();
        let right = self
            .resolve_term_handle(right)
            .ok_or(ErrorCode::NoSuchTermRegistered)?
            .clone();
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
    fn admit_theorem(&mut self, thm: Theorem) -> Handle {
        let fresh = self.issue_handle();
        self.theorems.insert(fresh, thm);
        fresh
    }

    /// Returns `Some(thm)` iff `handle` points-to a registered theorem in the
    /// runtime state's theorem table.
    #[inline]
    pub fn resolve_theorem_handle(&self, handle: &Handle) -> Option<&Theorem> {
        self.theorems.get(handle)
    }

    /// Returns `true` iff `handle` points to a registered theorem in the
    /// runtime state's theorem table.
    #[inline]
    pub fn is_theorem_registered(&self, handle: &Handle) -> bool {
        self.resolve_theorem_handle(handle).is_some()
    }

    pub fn register_reflexivity_theorem<T>(
        &mut self,
        hypotheses: T,
        trm: Handle,
    ) -> Result<Handle, ErrorCode>
    where
        T: Iterator<Item = Handle> + Clone,
    {
        if !self.is_proposition(&trm)? {
            return Err(ErrorCode::NotAProposition);
        }

        for c in hypotheses.clone() {
            if !self.is_proposition(&c)? {
                return Err(ErrorCode::NotAProposition);
            }
        }

        let conclusion = self.register_equality(trm.clone(), trm.clone())?;

        Ok(self.admit_theorem(Theorem::new(hypotheses, conclusion)))
    }

    pub fn register_symmetry_theorem(&mut self, handle: &Handle) -> Result<Handle, ErrorCode> {
        let thm = self
            .resolve_theorem_handle(handle)
            .ok_or(ErrorCode::NoSuchTheoremRegistered)?
            .clone();

        let (left, right) = self.split_equality(thm.conclusion())?;

        let left = left.clone();
        let right = right.clone();

        let conclusion = self.register_equality(right, left)?;
        let hypotheses = thm.hypotheses().iter().cloned();

        Ok(self.admit_theorem(Theorem::new(hypotheses, conclusion)))
    }

    pub fn register_transitivity_theorem(
        &mut self,
        left: &Handle,
        right: &Handle,
    ) -> Result<Handle, ErrorCode> {
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

        let hypotheses = left.hypotheses().iter().cloned();

        let (left, mid0) = self.split_equality(left.conclusion())?;
        let (mid1, right) = self.split_equality(right.conclusion())?;

        if mid0 != mid1 {
            return Err(ErrorCode::ShapeMismatch);
        }

        let left = left.clone();
        let right = right.clone();

        let conclusion = self.register_equality(left, right)?;

        Ok(self.admit_theorem(Theorem::new(hypotheses, conclusion)))
    }

    pub fn register_application_congruence_theorem(
        &mut self,
        left: &Handle,
        right: &Handle,
    ) -> Result<Handle, ErrorCode> {
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

        let (fun_left, fun_right) = self.split_equality(left.conclusion())?;
        let (arg_left, arg_right) = self.split_equality(right.conclusion())?;

        let fun_left = fun_left.clone();
        let fun_right = fun_right.clone();
        let arg_left = arg_left.clone();
        let arg_right = arg_right.clone();

        let left = self.register_application(fun_left, arg_left)?;
        let right = self.register_application(fun_right, arg_right)?;

        let conclusion = self.register_equality(left, right)?;

        Ok(self.admit_theorem(Theorem::new(hypotheses.iter().cloned(), conclusion)))
    }

    pub fn register_lambda_congruence_theorem<T>(
        &mut self,
        name: T,
        _type: Handle,
        handle: &Handle,
    ) -> Result<Handle, ErrorCode>
    where
        T: Into<Name> + Clone,
    {
        if !self.type_handle_is_registered(&_type) {
            return Err(ErrorCode::NoSuchTypeRegistered);
        }

        let thm = self
            .resolve_theorem_handle(handle)
            .ok_or(ErrorCode::NoSuchTheoremRegistered)?
            .clone();

        let (left, right) = self.split_equality(thm.conclusion())?;

        let left = left.clone();
        let right = right.clone();

        let lhandle = self.register_lambda(name.clone(), _type.clone(), left)?;
        let rhandle = self.register_lambda(name, _type, right)?;

        let conclusion = self.register_equality(lhandle, rhandle)?;
        let hypotheses = thm.hypotheses().iter().cloned();

        Ok(self.admit_theorem(Theorem::new(hypotheses, conclusion)))
    }

    pub fn register_beta_theorem<T>(
        &mut self,
        hypotheses: T,
        application: Handle,
    ) -> Result<Handle, ErrorCode>
    where
        T: Iterator<Item = Handle> + Clone,
    {
        for c in hypotheses.clone() {
            if !self.is_proposition(&c)? {
                return Err(ErrorCode::NotAProposition);
            }
        }

        let (lhs, rhs) = self.split_application(&application)?;

        let lhs = lhs.clone();
        let rhs = rhs.clone();

        let (name, _type, body) = self.split_lambda(&lhs)?;

        let name = name.clone();
        let rhs = rhs.clone();
        let body = body.clone();

        let subst = self.substitution(&body, vec![(name, rhs)].iter().cloned())?;
        let conclusion = self.register_equality(application, subst)?;

        Ok(self.admit_theorem(Theorem::new(hypotheses, conclusion)))
    }

    pub fn register_eta_theorem<T>(
        &mut self,
        hypotheses: T,
        lambda: Handle,
    ) -> Result<Handle, ErrorCode>
    where
        T: Iterator<Item = Handle> + Clone,
    {
        for c in hypotheses.clone() {
            if !self.is_proposition(&c)? {
                return Err(ErrorCode::NotAProposition);
            }
        }

        let (name0, _type, body) = self.split_lambda(&lambda)?;
        let (func, var) = self.split_application(body)?;

        let (name1, _type) = self.split_variable(var)?;

        if name0 != name1 {
            return Err(ErrorCode::ShapeMismatch);
        }

        if self.term_fv(func)?.contains(&(name1, _type)) {
            return Err(ErrorCode::ShapeMismatch);
        }

        let body = body.clone();

        let conclusion = self.register_equality(lambda, body)?;

        Ok(self.admit_theorem(Theorem::new(hypotheses, conclusion)))
    }

    pub fn register_equality_introduction_theorem(
        &mut self,
        left: &Handle,
        right: &Handle,
    ) -> Result<Handle, ErrorCode> {
        let left = self
            .resolve_theorem_handle(left)
            .ok_or(ErrorCode::NoSuchTheoremRegistered)?
            .clone();
        let right = self
            .resolve_theorem_handle(right)
            .ok_or(ErrorCode::NoSuchTheoremRegistered)?
            .clone();

        let (left0, right0) = self.split_implication(left.conclusion())?;
        let (left1, right1) = self.split_implication(right.conclusion())?;

        if left0 != right1 || left1 != right0 {
            return Err(ErrorCode::ShapeMismatch);
        }

        let left0 = left0.clone();
        let right1 = right1.clone();

        let mut hypotheses = left.hypotheses().clone();
        let mut merge = right.hypotheses().clone();

        hypotheses.append(&mut merge);

        let conclusion = self.register_equality(left0, right1)?;

        Ok(self.admit_theorem(Theorem::new(hypotheses.iter().cloned(), conclusion)))
    }

    pub fn register_equality_elimination_theorem(
        &mut self,
        handle: &Handle,
    ) -> Result<Handle, ErrorCode> {
        let thm = self
            .resolve_theorem_handle(handle)
            .ok_or(ErrorCode::NoSuchTheoremRegistered)?
            .clone();

        let (left, right) = self
            .split_equality(thm.conclusion())
            .map_err(|_e| ErrorCode::ShapeMismatch)?;

        let left = left.clone();
        let right = right.clone();

        if !self.is_proposition(&left)? {
            return Err(ErrorCode::NotAProposition);
        }

        let conclusion = self.register_implication(left, right)?;
        let hypotheses = thm.hypotheses().iter().cloned();

        Ok(self.admit_theorem(Theorem::new(hypotheses, conclusion)))
    }

    pub fn register_truth_introduction_theorem<T>(
        &mut self,
        hypotheses: T,
    ) -> Result<Handle, ErrorCode>
    where
        T: Iterator<Item = Handle> + Clone,
    {
        for c in hypotheses.clone() {
            if !self.is_term_registered(&c) {
                return Err(ErrorCode::NoSuchTermRegistered);
            }
        }

        let conclusion =
            self.register_constant_at_default_type(PREALLOCATED_HANDLE_CONSTANT_TRUE)?;

        Ok(self.admit_theorem(Theorem::new(hypotheses, conclusion)))
    }

    pub fn register_false_elimination_theorem(
        &mut self,
        thm: &Handle,
        conclusion: Handle,
    ) -> Result<Handle, ErrorCode> {
        let thm = self
            .resolve_theorem_handle(thm)
            .ok_or(ErrorCode::NoSuchTheoremRegistered)?
            .clone();

        if !self.is_term_registered(&conclusion) {
            return Err(ErrorCode::NoSuchTermRegistered);
        }

        if !self.is_proposition(&conclusion)? {
            return Err(ErrorCode::NotAProposition);
        }

        if !self.is_false(thm.conclusion())? {
            return Err(ErrorCode::ShapeMismatch);
        }

        let hypotheses = thm.hypotheses().iter().cloned();

        Ok(self.admit_theorem(Theorem::new(hypotheses, conclusion)))
    }

    pub fn register_conjunction_introduction_theorem(
        &mut self,
        left: &Handle,
        right: &Handle,
    ) -> Result<Handle, ErrorCode> {
        let left = self
            .resolve_theorem_handle(left)
            .ok_or(ErrorCode::NoSuchTheoremRegistered)?
            .clone();
        let right = self
            .resolve_theorem_handle(right)
            .ok_or(ErrorCode::NoSuchTheoremRegistered)?
            .clone();

        let conclusion =
            self.register_conjunction(left.conclusion().clone(), right.conclusion().clone())?;

        let mut hypotheses = left.hypotheses().clone();
        let mut merge = right.hypotheses().clone();

        hypotheses.append(&mut merge);

        Ok(self.admit_theorem(Theorem::new(hypotheses.iter().cloned(), conclusion)))
    }

    pub fn register_conjunction_elimination0_theorem(
        &mut self,
        handle: &Handle,
    ) -> Result<Handle, ErrorCode> {
        let thm = self
            .resolve_theorem_handle(handle)
            .ok_or(ErrorCode::NoSuchTheoremRegistered)?
            .clone();

        let (left, _right) = self
            .split_conjunction(thm.conclusion())
            .map_err(|_e| ErrorCode::ShapeMismatch)?;

        let conclusion = left.clone();
        let hypotheses = thm.hypotheses().iter().cloned();

        Ok(self.admit_theorem(Theorem::new(hypotheses, conclusion)))
    }

    pub fn register_conjunction_elimination1_theorem(
        &mut self,
        handle: &Handle,
    ) -> Result<Handle, ErrorCode> {
        let thm = self
            .resolve_theorem_handle(handle)
            .ok_or(ErrorCode::NoSuchTheoremRegistered)?
            .clone();

        let (_left, right) = self
            .split_conjunction(thm.conclusion())
            .map_err(|_e| ErrorCode::ShapeMismatch)?;

        let conclusion = right.clone();
        let hypotheses = thm.hypotheses().iter().cloned();

        Ok(self.admit_theorem(Theorem::new(hypotheses, conclusion)))
    }

    pub fn register_disjunction_introduction0_theorem(
        &mut self,
        handle: &Handle,
        term: Handle,
    ) -> Result<Handle, ErrorCode> {
        let thm = self
            .resolve_theorem_handle(handle)
            .ok_or(ErrorCode::NoSuchTheoremRegistered)?
            .clone();

        if !self.is_proposition(&term)? {
            return Err(ErrorCode::NotAProposition);
        }

        let conclusion = self.register_disjunction(thm.conclusion().clone(), term)?;
        let hypotheses = thm.hypotheses().iter().cloned();

        Ok(self.admit_theorem(Theorem::new(hypotheses, conclusion)))
    }

    pub fn register_disjunction_introduction1_theorem(
        &mut self,
        handle: &Handle,
        term: Handle,
    ) -> Result<Handle, ErrorCode> {
        let thm = self
            .resolve_theorem_handle(handle)
            .ok_or(ErrorCode::NoSuchTheoremRegistered)?
            .clone();

        if !self.is_proposition(&term)? {
            return Err(ErrorCode::NotAProposition);
        }

        let conclusion = self.register_disjunction(term, thm.conclusion().clone())?;
        let hypotheses = thm.hypotheses().iter().cloned();

        Ok(self.admit_theorem(Theorem::new(hypotheses, conclusion)))
    }

    pub fn register_disjunction_elimination_theorem(
        &mut self,
        left: &Handle,
        mid: &Handle,
        right: &Handle,
    ) -> Result<Handle, ErrorCode> {
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

        let (phi, psi) = self.split_disjunction(left.conclusion())?;

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
        let hypotheses = left.hypotheses().iter().cloned();

        Ok(self.admit_theorem(Theorem::new(hypotheses, conclusion)))
    }

    pub fn register_implication_introduction_theorem(
        &mut self,
        handle: &Handle,
        intro: &Handle,
    ) -> Result<Handle, ErrorCode> {
        let thm = self
            .resolve_theorem_handle(handle)
            .ok_or(ErrorCode::NoSuchTheoremRegistered)?
            .clone();

        if !self.is_proposition(intro)? {
            return Err(ErrorCode::NotAProposition);
        }

        if !thm.hypotheses().contains(intro) {
            return Err(ErrorCode::ShapeMismatch);
        }

        let conclusion = self.register_implication(intro.clone(), thm.conclusion().clone())?;
        let hypotheses = thm.hypotheses().iter().filter(|h| *h != intro).cloned();

        Ok(self.admit_theorem(Theorem::new(hypotheses, conclusion)))
    }

    pub fn register_implication_elimination_theorem(
        &mut self,
        left: &Handle,
        right: &Handle,
    ) -> Result<Handle, ErrorCode> {
        let left = self
            .resolve_theorem_handle(left)
            .ok_or(ErrorCode::NoSuchTheoremRegistered)?
            .clone();
        let right = self
            .resolve_theorem_handle(right)
            .ok_or(ErrorCode::NoSuchTheoremRegistered)?
            .clone();

        let (hyp, conc) = self
            .split_implication(left.conclusion())
            .map_err(|_e| ErrorCode::ShapeMismatch)?;

        if hyp != right.conclusion() {
            return Err(ErrorCode::ShapeMismatch);
        }

        let conc = conc.clone();

        let mut hypotheses = left.hypotheses().clone();
        let mut merge = right.hypotheses().clone();

        hypotheses.append(&mut merge);

        Ok(self.admit_theorem(Theorem::new(hypotheses.iter().cloned(), conc)))
    }

    pub fn register_substitution_theorem<T>(
        &mut self,
        handle: &Handle,
        sigma: T,
    ) -> Result<Handle, ErrorCode>
    where
        T: Iterator<Item = (Name, Handle)> + Clone,
    {
        let thm = self
            .resolve_theorem_handle(handle)
            .ok_or(ErrorCode::NoSuchTheoremRegistered)?
            .clone();

        let conclusion = self.substitution(thm.conclusion(), sigma.clone())?;
        let mut hypotheses = vec![];

        for h in thm.hypotheses().iter() {
            hypotheses.push(self.substitution(h, sigma.clone())?);
        }

        Ok(self.admit_theorem(Theorem::new(hypotheses.iter().cloned(), conclusion)))
    }

    pub fn register_instantiation_theorem<T>(
        &mut self,
        handle: &Handle,
        sigma: T,
    ) -> Result<Handle, ErrorCode>
    where
        T: Iterator<Item = (Name, Handle)> + Clone,
    {
        let thm = self
            .resolve_theorem_handle(handle)
            .ok_or(ErrorCode::NoSuchTheoremRegistered)?
            .clone();

        let conclusion = self.instantiation(thm.conclusion(), sigma.clone())?;
        let mut hypotheses = vec![];

        for h in thm.hypotheses().iter() {
            hypotheses.push(self.instantiation(h, sigma.clone())?);
        }

        Ok(self.admit_theorem(Theorem::new(hypotheses.iter().cloned(), conclusion)))
    }

    pub fn register_universal_elimination_theorem(
        &mut self,
        handle: &Handle,
        trm: Handle,
    ) -> Result<Handle, ErrorCode> {
        unimplemented!()
    }

    pub fn register_universal_introduction_theorem<T>(
        &mut self,
        handle: &Handle,
        name: T,
    ) -> Result<Handle, ErrorCode>
    where
        T: Into<Name>,
    {
        unimplemented!()
    }

    pub fn register_existential_introduction_theorem(
        &mut self,
        handle: &Handle,
        trm: Handle,
    ) -> Result<Handle, ErrorCode> {
        unimplemented!()
    }

    ////////////////////////////////////////////////////////////////////////////
    // Modifying the global theory.
    ////////////////////////////////////////////////////////////////////////////

    pub fn register_new_definition(
        &mut self,
        defn: &Handle,
    ) -> Result<(Handle, Handle), ErrorCode> {
        /* 1. Check the body of the definition exists, and it has a type. */
        let tau = self.infer_type(defn)?;

        /* 2. Add the new constant, giving it the type inferred previously. */
        let cnst_handle = self.issue_handle();
        self.constants.insert(cnst_handle.clone(), tau.clone());

        /* 3. Lift the registered constant into a term. */
        let cnst = self
            .register_constant_at_default_type(cnst_handle)
            .expect(PRIMITIVE_CONSTRUCTION_ERROR);

        /* 4. Construct the definitional theorem. */
        let stmt = self
            .register_equality(cnst, defn.clone())
            .expect(PRIMITIVE_CONSTRUCTION_ERROR);

        /* 5. Register the definitional theorem. */
        let thm = self.admit_theorem(Theorem::new(Vec::new().iter().cloned(), stmt));

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
            .type_former_handle_resolve(&PREALLOCATED_HANDLE_TYPE_FORMER_PROP)
            .is_some());
        assert!(state
            .type_former_handle_resolve(&PREALLOCATED_HANDLE_TYPE_FORMER_ARROW)
            .is_some());
    }

    /// Tests all primitive constants are registered in the initial theory.
    #[test]
    pub fn initial_theory1() {
        let state = RuntimeState::new();

        assert!(state
            .constant_handle_resolve(&PREALLOCATED_HANDLE_CONSTANT_EXISTS)
            .is_some());
        assert!(state
            .constant_handle_resolve(&PREALLOCATED_HANDLE_CONSTANT_FORALL)
            .is_some());
        assert!(state
            .constant_handle_resolve(&PREALLOCATED_HANDLE_CONSTANT_IMPLICATION)
            .is_some());
        assert!(state
            .constant_handle_resolve(&PREALLOCATED_HANDLE_CONSTANT_CONJUNCTION)
            .is_some());
        assert!(state
            .constant_handle_resolve(&PREALLOCATED_HANDLE_CONSTANT_DISJUNCTION)
            .is_some());
        assert!(state
            .constant_handle_resolve(&PREALLOCATED_HANDLE_CONSTANT_TRUE)
            .is_some());
        assert!(state
            .constant_handle_resolve(&PREALLOCATED_HANDLE_CONSTANT_FALSE)
            .is_some());
        assert!(state
            .constant_handle_resolve(&PREALLOCATED_HANDLE_CONSTANT_EQUALITY)
            .is_some());
        assert!(state
            .constant_handle_resolve(&PREALLOCATED_HANDLE_CONSTANT_NEGATION)
            .is_some());
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
            .is_some());
        assert!(state
            .resolve_term_handle(&PREALLOCATED_HANDLE_TERM_FORALL)
            .is_some());
        assert!(state
            .resolve_term_handle(&PREALLOCATED_HANDLE_TERM_IMPLICATION)
            .is_some());
        assert!(state
            .resolve_term_handle(&PREALLOCATED_HANDLE_TERM_CONJUNCTION)
            .is_some());
        assert!(state
            .resolve_term_handle(&PREALLOCATED_HANDLE_TERM_DISJUNCTION)
            .is_some());
        assert!(state
            .resolve_term_handle(&PREALLOCATED_HANDLE_TERM_TRUE)
            .is_some());
        assert!(state
            .resolve_term_handle(&PREALLOCATED_HANDLE_TERM_FALSE)
            .is_some());
        assert!(state
            .resolve_term_handle(&PREALLOCATED_HANDLE_TERM_EQUALITY)
            .is_some());
        assert!(state
            .resolve_term_handle(&PREALLOCATED_HANDLE_TERM_NEGATION)
            .is_some());
    }

    ////////////////////////////////////////////////////////////////////////////
    // Free-variable tests.
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    pub fn free_variables0() {
        let mut state = RuntimeState::new();

        let v = state
            .register_variable("a", PREALLOCATED_HANDLE_TYPE_PROP)
            .unwrap();

        let fvs = state.term_fv(&v).unwrap();

        assert_eq!(
            fvs,
            vec![(&String::from("a"), &PREALLOCATED_HANDLE_TYPE_PROP)]
        );
    }

    #[test]
    pub fn free_variables1() {
        let state = RuntimeState::new();

        let fvs = state.term_fv(&PREALLOCATED_HANDLE_TERM_TRUE).unwrap();

        assert!(fvs.is_empty());
    }

    #[test]
    pub fn free_variables2() {
        let mut state = RuntimeState::new();

        let v = state
            .register_variable("a", PREALLOCATED_HANDLE_TYPE_PROP)
            .unwrap();
        let l = state
            .register_lambda("a", PREALLOCATED_HANDLE_TYPE_PROP, v)
            .unwrap();

        let fvs = state.term_fv(&l).unwrap();

        assert!(fvs.is_empty());
    }

    #[test]
    pub fn free_variables3() {
        let mut state = RuntimeState::new();

        let v = state
            .register_variable("a", PREALLOCATED_HANDLE_TYPE_BINARY_PREDICATE)
            .unwrap();
        let l = state
            .register_lambda("a", PREALLOCATED_HANDLE_TYPE_PROP, v)
            .unwrap();

        let fvs = state.term_fv(&l).unwrap();

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
            .register_lambda(
                "a",
                PREALLOCATED_HANDLE_TYPE_PROP,
                PREALLOCATED_HANDLE_TERM_TRUE,
            )
            .unwrap();

        let fvs = state.term_fv(&l).unwrap();

        assert!(fvs.is_empty())
    }

    #[test]
    pub fn free_variables5() {
        let mut state = RuntimeState::new();

        let l = state
            .register_lambda(
                "a",
                PREALLOCATED_HANDLE_TYPE_PROP,
                PREALLOCATED_HANDLE_TERM_TRUE,
            )
            .unwrap();
        let v = state
            .register_variable("v", PREALLOCATED_HANDLE_TYPE_PROP)
            .unwrap();
        let t = state.register_application(l, v).unwrap();

        let fvs = state.term_fv(&t).unwrap();

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
            .register_variable("a", PREALLOCATED_HANDLE_TYPE_PROP)
            .unwrap();

        assert!(state.is_alpha_equivalent(&v, &v).unwrap());
    }

    #[test]
    pub fn alpha_equivalence1() {
        let mut state = RuntimeState::new();

        let v = state
            .register_variable("a", PREALLOCATED_HANDLE_TYPE_PROP)
            .unwrap();
        let q = state
            .register_variable("b", PREALLOCATED_HANDLE_TYPE_PROP)
            .unwrap();

        assert!(!state.is_alpha_equivalent(&v, &q).unwrap());
    }

    #[test]
    pub fn alpha_equivalence2() {
        let mut state = RuntimeState::new();

        let v = state
            .register_variable("a", PREALLOCATED_HANDLE_TYPE_PROP)
            .unwrap();
        let q = state
            .register_variable("a", PREALLOCATED_HANDLE_TYPE_BINARY_PREDICATE)
            .unwrap();

        assert!(!state.is_alpha_equivalent(&v, &q).unwrap());
    }

    #[test]
    pub fn alpha_equivalence3() {
        let mut state = RuntimeState::new();

        let v = state
            .register_variable("a", PREALLOCATED_HANDLE_TYPE_PROP)
            .unwrap();
        let l = state
            .register_lambda("a", PREALLOCATED_HANDLE_TYPE_PROP, v.clone())
            .unwrap();
        let c = state.register_application(l, v).unwrap();

        assert!(state.is_alpha_equivalent(&c, &c).unwrap());
    }

    #[test]
    pub fn alpha_equivalence4() {
        let mut state = RuntimeState::new();

        let v0 = state
            .register_variable("a", PREALLOCATED_HANDLE_TYPE_PROP)
            .unwrap();
        let l0 = state
            .register_lambda("a", PREALLOCATED_HANDLE_TYPE_PROP, v0.clone())
            .unwrap();
        let c0 = state.register_application(l0, v0).unwrap();

        let v1 = state
            .register_variable("b", PREALLOCATED_HANDLE_TYPE_PROP)
            .unwrap();
        let l1 = state
            .register_lambda("b", PREALLOCATED_HANDLE_TYPE_PROP, v1.clone())
            .unwrap();
        let c1 = state.register_application(l1, v1).unwrap();

        assert!(state.is_alpha_equivalent(&c0, &c1).unwrap());
    }

    ////////////////////////////////////////////////////////////////////////////
    // Substitution tests.
    ////////////////////////////////////////////////////////////////////////////

    ////////////////////////////////////////////////////////////////////////////
    // Type-checking tests.
    ////////////////////////////////////////////////////////////////////////////

    pub fn type_checking0() {}

    ////////////////////////////////////////////////////////////////////////////
    // Inference tests.
    ////////////////////////////////////////////////////////////////////////////
}
