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

use crate::wasmi::{
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
use std::hint::unreachable_unchecked;

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
    /// Will raise a kernel panic if any of the manipulated tyoes are malformed.
    pub fn type_instantiate<T>(&mut self, tau: &Handle, sigma: T) -> Result<Handle, ErrorCode>
    where
        T: Iterator<Item = (Name, Handle)>,
    {
        let mut tau = self
            .resolve_type_handle(tau)
            .ok_or(ErrorCode::NoSuchTypeRegistered)?
            .clone();

        let sigma = sigma.map(|(domain, range)| {
            (
                domain,
                self.resolve_type_handle(&range)
                    .ok_or(ErrorCode::NoSuchTypeRegistered)?,
            )
        });

        for (domain, range) in sigma {
            match tau {
                Type::Variable { ref name } => {
                    if name == &domain {
                        tau = range.clone();
                    }
                }
                Type::Combination { former, arguments } => {
                    let arguments = arguments
                        .iter()
                        .map(|a| self.resolve_type_handle(a).expect(DANGLING_HANDLE_ERROR))
                        .collect();
                    tau = Type::Combination { former, arguments }
                }
            }
        }

        Ok(self.admit_type(tau))
    }

    ////////////////////////////////////////////////////////////////////////////
    // Constant related material.
    ////////////////////////////////////////////////////////////////////////////

    pub fn register_constant(&mut self, handle: Handle) -> Result<Handle, ErrorCode> {
        if !self.is_type_registered(&handle) {
            return Err(ErrorCode::NoSuchTypeRegistered);
        }

        let fresh = self.issue_handle();
        self.constants.insert(fresh, handle);
        Ok(fresh)
    }

    #[inline]
    pub fn resolve_constant_handle(&self, handle: &Handle) -> Option<&Handle> {
        self.constants.get(handle)
    }

    #[inline]
    pub fn is_constant_registered(&self, handle: &Handle) -> bool {
        self.constant(handle).is_ok()
    }

    ////////////////////////////////////////////////////////////////////////////
    // Term related material.
    ////////////////////////////////////////////////////////////////////////////

    fn admit_term(&mut self, trm: Term) -> Handle {
        for (handle, registered) in self.terms.iter() {
            if self.is_alpha_equivalent_inner(&trm, &registered) {
                return *handle;
            }
        }

        let fresh = self.issue_handle();
        self.terms.insert(fresh, trm);
        fresh
    }

    pub fn register_variable<T>(&mut self, name: T, handle: Handle) -> Result<Handle, ErrorCode>
    where
        T: Into<Name>,
    {
        if !self.is_type_registered(&handle) {
            return Err(ErrorCode::NoSuchTypeRegistered);
        }

        Ok(self.admit_term(Term::variable(name, handle)))
    }

    pub fn register_constant_at_default_type(
        &mut self,
        handle: Handle,
    ) -> Result<Handle, ErrorCode> {
        if !self.is_constant_registered(&handle) {
            return Err(ErrorCode::NoSuchConstantRegistered);
        }

        Ok(self.admit_term(Term::constant(handle, None)))
    }

    pub fn register_constant_at_constrained_type<T>(
        &mut self,
        handle: Handle,
        type_substitution: T,
    ) -> Result<Handle, ErrorCode>
    where
        T: Iterator<Item = (Name, Handle)>,
    {
        if let Some(tau) = self.resolve_constant_handle(&handle) {
            let tau = self.type_instantiate(tau, type_substitution)?;
            Ok(self.admit_term(Term::constant(handle, Some(tau))))
        } else {
            Err(ErrorCode::NoSuchConstantRegistered)
        }
    }

    pub fn register_application(
        &mut self,
        left: Handle,
        right: Handle,
    ) -> Result<Handle, ErrorCode> {
        unimplemented!()
    }

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

    #[inline]
    pub fn resolve_term_handle(&self, handle: &Handle) -> Option<&Term> {
        self.terms.get(handle)
    }

    #[inline]
    pub fn is_term_registered(&self, handle: &Handle) -> bool {
        self.resolve_term_handle(handle).is_some()
    }

    pub fn split_variable(&self, handle: &Handle) -> Result<(&Name, &Handle), ErrorCode> {
        unimplemented!()
    }

    pub fn split_constant(&self, handle: &Handle) -> Result<(&Handle, &Handle), ErrorCode> {
        unimplemented!()
    }

    pub fn split_application(&self, handle: &Handle) -> Result<(&Handle, &Handle), ErrorCode> {
        unimplemented!()
    }

    pub fn split_lambda(&self, handle: &Handle) -> Result<(&Name, &Handle, &Handle), ErrorCode> {
        unimplemented!()
    }

    #[inline]
    pub fn is_variable(&self, handle: &Handle) -> Result<bool, ErrorCode> {
        Ok(self
            .resolve_term_handle(handle)
            .ok_or(ErrorCode::NoSuchTermRegistered)?
            .split_variable()
            .is_some())
    }

    #[inline]
    pub fn is_constant(&self, handle: &Handle) -> Result<bool, ErrorCode> {
        Ok(self
            .resolve_term_handle(handle)
            .ok_or(ErrorCode::NoSuchTermRegistered)?
            .split_constant()
            .is_some())
    }

    #[inline]
    pub fn is_application(&self, handle: &Handle) -> Result<bool, ErrorCode> {
        Ok(self
            .resolve_term_handle(handle)
            .ok_or(ErrorCode::NoSuchTermRegistered)?
            .split_application()
            .is_some())
    }

    #[inline]
    pub fn is_lambda(&self, handle: &Handle) -> Result<bool, ErrorCode> {
        Ok(self
            .resolve_term_handle(handle)
            .ok_or(ErrorCode::NoSuchTermRegistered)?
            .split_lambda()
            .is_some())
    }

    pub fn term_ftv(&self, handle: &Handle) -> Result<Vec<&Name>, ErrorCode> {
        unimplemented!()
    }

    pub fn term_fv(&self, handle: &Handle) -> Result<Vec<&Name>, ErrorCode> {
        unimplemented!()
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

    fn admit_theorem(&mut self, thm: Theorem) -> Handle {
        let fresh = self.issue_handle();
        self.theorems.insert(fresh, thm);
        fresh
    }

    #[inline]
    pub fn resolve_theorem_handle(&self, handle: &Handle) -> Option<&Theorem> {
        self.theorems.get(handle)
    }

    #[inline]
    pub fn is_theorem_registered(&self, handle: &Handle) -> bool {
        self.resolve_theorem_handle(handle).is_some()
    }

    pub fn register_reflexivity_theorem<T>(&mut self, context: T) -> Result<Handle, ErrorCode>
    where
        T: Iterator<Item = Handle>,
    {
        unimplemented!()
    }

    pub fn register_symmetry_theorem(&mut self, handle: &Handle) -> Result<Handle, ErrorCode> {
        unimplemented!()
    }

    pub fn register_transisivity_theorem(
        &mut self,
        left: &Handle,
        right: &Handle,
    ) -> Result<Handle, ErrorCode> {
        unimplemented!()
    }

    pub fn register_application_congruence_theorem(
        &mut self,
        left: &Handle,
        right: &Handle,
    ) -> Result<Handle, ErrorCode> {
        unimplemented!()
    }

    pub fn register_lambda_congruence_theorem<T>(
        &mut self,
        name: T,
        _type: Handle,
        handle: &Handle,
    ) -> Result<Handle, ErrorCode>
    where
        T: Into<Name>,
    {
        unimplemented!()
    }

    pub fn register_beta_theorem(
        &mut self,
        left: &Handle,
        right: &Handle,
    ) -> Result<Handle, ErrorCode> {
        unimplemented!()
    }

    pub fn register_eta_theorem(
        &mut self,
        left: &Handle,
        right: &Handle,
    ) -> Result<Handle, ErrorCode> {
        unimplemented!()
    }

    pub fn register_equality_introduction_theorem(
        &mut self,
        left: &Handle,
        right: &Handle,
    ) -> Result<Handle, ErrorCode> {
        unimplemented!()
    }

    pub fn register_equality_elimination_theorem(
        &mut self,
        handle: &Handle,
    ) -> Result<Handle, ErrorCode> {
        unimplemented!()
    }

    pub fn register_truth_introduction_theorem<T>(
        &mut self,
        context: T,
    ) -> Result<Handle, ErrorCode>
    where
        T: Iterator<Item = Handle>,
    {
        unimplemented!()
    }

    pub fn register_false_elimination_theorem(
        &mut self,
        thm: &Handle,
        term: &Handle,
    ) -> Result<Handle, ErrorCode> {
        unimplemented!()
    }

    pub fn register_conjunction_introduction_theorem(
        &mut self,
        left: &Handle,
        right: &Handle,
    ) -> Result<Handle, ErrorCode> {
        unimplemented!()
    }

    pub fn register_conjunction_elimination0_theorem(
        &mut self,
        handle: &Handle,
    ) -> Result<Handle, ErrorCode> {
        unimplemented!()
    }

    pub fn register_conjunction_elimination1_theorem(
        &mut self,
        handle: &Handle,
    ) -> Result<Handle, ErrorCode> {
        unimplemented!()
    }

    pub fn register_disjunction_introduction0_theorem(
        &mut self,
        handle: &Handle,
        term: &Handle,
    ) -> Result<Handle, ErrorCode> {
        unimplemented!()
    }

    pub fn register_disjunction_introduction1_theorem(
        &mut self,
        handle: &Handle,
        term: &Handle,
    ) -> Result<Handle, ErrorCode> {
        unimplemented!()
    }

    pub fn register_disjunction_elimination_theorem(
        &mut self,
        left: &Handle,
        mid: &Handle,
        right: &Handle,
    ) -> Result<Handle, ErrorCode> {
        unimplemented!()
    }

    pub fn register_implication_introduction_theorem(
        &mut self,
        handle: &Handle,
        intro: &Handle,
    ) -> Result<Handle, ErrorCode> {
        unimplemented!()
    }

    pub fn register_implication_elimination_theorem(
        &mut self,
        left: &Handle,
        right: &Handle,
    ) -> Result<Handle, ErrorCode> {
        unimplemented!()
    }

    pub fn register_substitution_theorem<T>(
        &mut self,
        handle: &Handle,
        sigma: T,
    ) -> Result<Handle, ErrorCode>
    where
        T: Iterator<Item = (Name, Handle)>,
    {
        unimplemented!()
    }

    pub fn register_instantiation_theorem<T>(
        &mut self,
        handle: &Handle,
        sigma: T,
    ) -> Result<Handle, ErrorCode>
    where
        T: Iterator<Item = (Name, Handle)>,
    {
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
