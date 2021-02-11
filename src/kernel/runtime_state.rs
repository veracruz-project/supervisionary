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
        PREALLOCATED_HANDLE_CONSTANT_TRUE, PREALLOCATED_HANDLE_TERM_FALSE,
        PREALLOCATED_HANDLE_TERM_TRUE, PREALLOCATED_HANDLE_TYPE_ALPHA,
        PREALLOCATED_HANDLE_TYPE_BETA, PREALLOCATED_HANDLE_TYPE_BINARY_CONNECTIVE,
        PREALLOCATED_HANDLE_TYPE_BINARY_PREDICATE, PREALLOCATED_HANDLE_TYPE_FORMER_ARROW,
        PREALLOCATED_HANDLE_TYPE_FORMER_PROP, PREALLOCATED_HANDLE_TYPE_PROP,
        PREALLOCATED_HANDLE_TYPE_QUANTIFIER, PREALLOCATED_HANDLE_TYPE_UNARY_CONNECTIVE,
        PREALLOCATED_HANDLE_TYPE_UNARY_PREDICATE,
    },
    kernel_panic::{DANGLING_HANDLE_ERROR, HANDLE_EXHAUST_ERROR},
    name::Name,
    term::{Term, TERM_FALSE_CONSTANT, TERM_TRUE_CONSTANT},
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
    pub fn register_type_former<T>(&mut self, arity: T) -> Handle
    where
        T: Into<usize>,
    {
        let handle = self.issue_handle();
        self.type_formers.insert(handle, arity.into());
        handle
    }

    /// Returns the arity of the type-former pointed-to by `handle`.  Returns
    /// `Err(ErrorCode::NoSuchTypeFormerRegistered)` if `handle` does not
    /// point-to any such type-former registered with the runtime state.
    #[inline]
    pub fn resolve_type_former_handle(&self, handle: &Handle) -> Result<&usize, ErrorCode> {
        self.type_formers
            .get(handle)
            .ok_or(ErrorCode::NoSuchTypeFormerRegistered)
    }

    /// Returns `true` iff `handle` points to a type-former registered with the
    /// runtime state.
    #[inline]
    pub fn is_type_former_registered(&self, handle: &Handle) -> bool {
        self.resolve_type_former_handle(handle).is_ok()
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
    pub fn is_type_registered(&self, handle: &Handle) -> bool {
        self.resolve_type_handle(handle).is_some()
    }

    /// Registers a new type in the runtime state's type-table with a given
    /// name.  Returns the handle of the newly-allocated type (or the existing
    /// handle, if the type-variable already appears in the type-table).
    #[inline]
    pub fn register_type_variable<T>(&mut self, name: T) -> Handle
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
    pub fn register_type_combination(
        &mut self,
        former: Handle,
        arguments: Vec<Handle>,
    ) -> Result<Handle, ErrorCode> {
        if let Ok(arity) = self.resolve_type_former_handle(&former) {
            if !arguments.iter().all(|a| self.is_type_registered(a)) {
                return Err(ErrorCode::NoSuchTypeRegistered);
            }

            if arguments.len() != *arity {
                return Err(ErrorCode::MismatchedArity);
            }

            Ok(self.admit_type(Type::Combination { former, arguments }))
        } else {
            return Err(ErrorCode::NoSuchTypeFormerRegistered);
        }
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
    pub fn register_function_type(
        &mut self,
        domain: Handle,
        range: Handle,
    ) -> Result<Handle, ErrorCode> {
        if !self.is_type_registered(&domain) || !self.is_type_registered(&range) {
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
    pub fn split_type_variable(&self, handle: &Handle) -> Result<&Name, ErrorCode> {
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
    pub fn split_type_combination(
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
    pub fn split_function_type(&self, handle: &Handle) -> Result<(&Handle, &Handle), ErrorCode> {
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
    pub fn is_type_variable(&self, handle: &Handle) -> Result<bool, ErrorCode> {
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
    pub fn is_type_combination(&self, handle: &Handle) -> Result<bool, ErrorCode> {
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
    pub fn is_function_type(&self, handle: &Handle) -> Result<bool, ErrorCode> {
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
    pub fn type_instantiate<T>(&mut self, tau: &Handle, sigma: T) -> Result<Handle, ErrorCode>
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
                        let argument = self.type_instantiate(a, sigma.clone())?;
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
    pub fn register_constant(&mut self, handle: Handle) -> Result<Handle, ErrorCode> {
        if !self.is_type_registered(&handle) {
            return Err(ErrorCode::NoSuchTypeRegistered);
        }

        let fresh = self.issue_handle();
        self.constants.insert(fresh, handle);
        Ok(fresh)
    }

    /// Returns `Some(tau)` iff `handle` points to a registered constant, with
    /// type handle `tau`, in the runtime state's type-table.
    #[inline]
    pub fn resolve_constant_handle(&self, handle: &Handle) -> Option<&Handle> {
        self.constants.get(handle)
    }

    /// Returns `true` iff `handle` points-to a registered constant in the
    /// runtime state's constant table.
    #[inline]
    pub fn is_constant_registered(&self, handle: &Handle) -> bool {
        self.resolve_constant_handle(handle).is_some()
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
        if !self.is_type_registered(&handle) {
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
        if !self.is_constant_registered(&handle) {
            return Err(ErrorCode::NoSuchConstantRegistered);
        }

        Ok(self.admit_term(Term::constant(handle, None)))
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
        if let Some(tau) = self.clone().resolve_constant_handle(&handle) {
            let tau = self.type_instantiate(tau, type_substitution)?;
            Ok(self.admit_term(Term::constant(handle, Some(tau))))
        } else {
            Err(ErrorCode::NoSuchConstantRegistered)
        }
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

        let (dom, rng) = self.split_function_type(&ltau)?;

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
        if !self.is_type_registered(&_type) {
            return Err(ErrorCode::NoSuchTypeRegistered);
        }

        if !self.is_term_registered(&body) {
            return Err(ErrorCode::NoSuchTermRegistered);
        }

        Ok(self.admit_term(Term::lambda(name, _type, body)))
    }

    pub fn register_equality(&mut self, left: Handle, right: Handle) -> Result<Handle, ErrorCode> {
        unimplemented!()
    }

    pub fn register_disjunction(
        &mut self,
        left: Handle,
        right: Handle,
    ) -> Result<Handle, ErrorCode> {
        unimplemented!()
    }

    pub fn register_conjunction(
        &mut self,
        left: Handle,
        right: Handle,
    ) -> Result<Handle, ErrorCode> {
        unimplemented!()
    }

    pub fn register_implication(
        &mut self,
        left: Handle,
        right: Handle,
    ) -> Result<Handle, ErrorCode> {
        unimplemented!()
    }

    /// Returns `Some(trm)` iff `handle` points-to the term `trm` in the runtime
    /// state's term-table.
    #[inline]
    pub fn resolve_term_handle(&self, handle: &Handle) -> Option<&Term> {
        self.terms.get(handle)
    }

    /// Returns `true` iff `handle` points-to a registered term in the runtime
    /// state's term-table.
    #[inline]
    pub fn is_term_registered(&self, handle: &Handle) -> bool {
        self.resolve_term_handle(handle).is_some()
    }

    pub fn split_variable(&self, handle: &Handle) -> Result<(&Name, &Handle), ErrorCode> {
        let trm = self
            .resolve_term_handle(handle)
            .ok_or(ErrorCode::NoSuchTermRegistered)?;

        if let Term::Variable { name, _type } = trm {
            Ok((name, _type))
        } else {
            Err(ErrorCode::NotAVariable)
        }
    }

    pub fn split_constant(&self, handle: &Handle) -> Result<(&Handle, &Handle), ErrorCode> {
        let trm = self
            .resolve_term_handle(handle)
            .ok_or(ErrorCode::NoSuchTermRegistered)?;

        if let Term::Constant { handle, _type } = trm {
            if let Some(tau) = _type {
                return Ok((handle, tau));
            }

            let tau = self
                .resolve_constant_handle(handle)
                .expect(DANGLING_HANDLE_ERROR);

            Ok((handle, tau))
        } else {
            Err(ErrorCode::NotAConstant)
        }
    }

    pub fn split_application(&self, handle: &Handle) -> Result<(&Handle, &Handle), ErrorCode> {
        let trm = self
            .resolve_term_handle(handle)
            .ok_or(ErrorCode::NoSuchTermRegistered)?;

        if let Term::Application { left, right } = trm {
            Ok((left, right))
        } else {
            Err(ErrorCode::NotAnApplication)
        }
    }

    pub fn split_lambda(&self, handle: &Handle) -> Result<(&Name, &Handle, &Handle), ErrorCode> {
        let trm = self
            .resolve_term_handle(handle)
            .ok_or(ErrorCode::NoSuchTermRegistered)?;

        if let Term::Lambda { name, _type, body } = trm {
            Ok((name, _type, body))
        } else {
            Err(ErrorCode::NotALambda)
        }
    }

    pub fn split_equality(&self, handle: &Handle) -> Result<(&Handle, &Handle), ErrorCode> {
        unimplemented!()
    }

    pub fn split_disjunction(&self, handle: &Handle) -> Result<(&Handle, &Handle), ErrorCode> {
        unimplemented!()
    }

    pub fn split_conjunction(&self, handle: &Handle) -> Result<(&Handle, &Handle), ErrorCode> {
        unimplemented!()
    }

    pub fn split_implication(&self, handle: &Handle) -> Result<(&Handle, &Handle), ErrorCode> {
        unimplemented!()
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

    #[inline]
    pub fn is_true(&self, handle: &Handle) -> Result<bool, ErrorCode> {
        unimplemented!()
    }

    #[inline]
    pub fn is_false(&self, handle: &Handle) -> Result<bool, ErrorCode> {
        unimplemented!()
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

    /// Computes the *free type-variables* of the term pointed-to by the handle
    /// `handle` in the runtime state's term-table.
    ///
    /// # Errors
    ///
    /// Returns `Err(ErrorCode::NoSuchTermRegistered)` if `handle` does not
    /// point-to any term in the runtime state's term-table.
    pub fn term_ftv(&self, handle: &Handle) -> Result<Vec<&Name>, ErrorCode> {
        let trm = self
            .resolve_term_handle(handle)
            .ok_or(ErrorCode::NoSuchTermRegistered)?;

        let mut work_list = vec![trm];
        let mut ftv = vec![];

        while let Some(next) = work_list.pop() {
            match next {
                Term::Variable { _type, .. } => {
                    let mut fvs = self.type_ftv(_type).expect(DANGLING_HANDLE_ERROR);
                    ftv.append(&mut fvs);
                }
                Term::Constant { handle, _type, .. } => {
                    if let Some(tau) = _type {
                        let mut fvs = self.type_ftv(tau).expect(DANGLING_HANDLE_ERROR);
                        ftv.append(&mut fvs);
                    } else {
                        let tau = self
                            .resolve_constant_handle(handle)
                            .expect(DANGLING_HANDLE_ERROR);

                        let mut fvs = self.type_ftv(tau).expect(DANGLING_HANDLE_ERROR);
                        ftv.append(&mut fvs);
                    }
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

    pub fn term_fv(&self, handle: &Handle) -> Result<Vec<&Name>, ErrorCode> {
        let term = self
            .resolve_term_handle(handle)
            .ok_or(ErrorCode::NoSuchTermRegistered)?;

        match term {
            Term::Variable { name, .. } => Ok(vec![name]),
            Term::Constant { .. } => Ok(vec![]),
            Term::Application { left, right } => {
                let mut left = self.term_fv(left).expect(DANGLING_HANDLE_ERROR);
                let mut right = self.term_fv(right).expect(DANGLING_HANDLE_ERROR);

                left.append(&mut right);

                Ok(left)
            }
            Term::Lambda { name, body, .. } => {
                let body = self.term_fv(body).expect(DANGLING_HANDLE_ERROR);

                Ok(body.iter().filter(|v| **v != name).cloned().collect())
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
        unimplemented!()
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
        unimplemented!()
    }

    fn is_alpha_equivalent_inner(&mut self, left: &Term, right: &Term) -> Result<bool, ErrorCode> {
        unimplemented!()
    }

    pub fn is_alpha_equivalent(
        &mut self,
        left: &Handle,
        right: &Handle,
    ) -> Result<bool, ErrorCode> {
        unimplemented!()
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
        if !self.is_type_registered(&_type) {
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

        if self.term_fv(func)?.contains(&name1) {
            return Err(ErrorCode::ShapeMismatch);
        }

        let conclusion = self.register_equality(lambda, body.clone())?;

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
}

impl Default for RuntimeState {
    /// Creates a new, default runtime state with all primitive kernel objects
    /// registered.
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
        ]);

        let theorems = HashMap::from_iter(vec![]);

        RuntimeState {
            next_handle: 0,
            type_formers,
            types,
            constants,
            terms,
            theorems,
        }
    }
}
