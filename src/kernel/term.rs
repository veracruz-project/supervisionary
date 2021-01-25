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

use super::{
    _type::{_type, is_type_registered, register_type, Type},
    constant::{constant_type, is_constant_registered},
    error_code::ErrorCode,
    handle::{issue_handle, Handle},
};
use lazy_static::lazy_static;
use std::{collections::HashMap, sync::Mutex};

////////////////////////////////////////////////////////////////////////////////
// Miscellaneous material.
////////////////////////////////////////////////////////////////////////////////

/// The error message used when panicking if the lock on the term table cannot
/// be obtained.
const TABLE_LOCK_ERROR: &str = "Failed to obtain lock on term table.";

/// The error message used when panicking if a dangling handle is detected in a
/// term.
const DANGLING_HANDLE_ERROR: &str = "Kernel invariant failed: dangling handle.";

/// The error message used when panicking if a dangling handle is detected in a
/// term.
const PRIMITIVE_CONSTRUCTION_ERROR: &str =
    "Kernel invariant failed: failed to construct a kernel primitive.";

/// The default stem for fresh name generation if not explicitly over-ridden by
/// the caller.
const FRESH_NAME_STEM: &str = "f";

////////////////////////////////////////////////////////////////////////////////
// Names and related material.
////////////////////////////////////////////////////////////////////////////////

/// We use Strings to represent variable names.
pub type Name = String;

/// Fresh name generation, for e.g. implementing the capture-avoiding
/// substitution action.  Finds a name that is not contained in the `avoid` set
/// of names. If `base` is `Some(b)` for a name `b` then `b` is used as the stem
/// of the freshly-generated name, otherwise a default is used.
fn fresh<T, U>(base: Option<U>, mut avoid: T) -> Name
where
    T: Iterator<Item = Name>,
    U: Into<Name>,
{
    let mut counter = 0usize;

    let base = base
        .map(|b| b.into())
        .unwrap_or(String::from(FRESH_NAME_STEM));

    loop {
        let generated = format!("{}{}", base, counter);

        if !avoid.any(|x| x == generated) {
            return generated;
        } else {
            counter += 1;
        }
    }
}

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
    /// Constructs a new variable lambda-term from a name and a handle to a
    /// type.
    ///
    /// Returns `Err(ErrorCode::NoSuchTypeRegistered)` iff `tau` does not point
    /// to a registered type in the kernel's type-table.
    pub fn variable<T>(name: T, tau: Handle) -> Result<Self, ErrorCode>
    where
        T: Into<Name>,
    {
        if is_type_registered(&tau) {
            Ok(Term::Variable {
                name: name.into(),
                _type: tau,
            })
        } else {
            Err(ErrorCode::NoSuchTypeRegistered)
        }
    }

    /// Constructs a new constant lambda-term from a handle to a registered
    /// constant in the kernel's constant table.  Note that this function will
    /// create a constant using the type registered in the constant-table, if
    /// registered.
    ///
    /// Returns `Err(ErrorCode::NoSuchConstantRegistered)` iff `constant` does
    /// not point to any registered constant in the constant-table.
    pub fn constant_at_registered_type(constant: Handle) -> Result<Self, ErrorCode> {
        if is_constant_registered(&constant) {
            Ok(Term::Constant {
                handle: constant,
                _type: None,
            })
        } else {
            Err(ErrorCode::NoSuchConstantRegistered)
        }
    }

    /// Constructs a constant lambda-term from a handle to a registered constant
    /// in the kernel's constant table, and a type-substitution represented as a
    /// list of pairs of type-variables and handle to types.
    ///
    /// Returns `Err(ErrorCode::NoSuchConstantRegistered)` iff `constant` does
    /// not point to any registered constant in the constant-table.  Returns
    /// `Err(ErrorCode::NoSuchTypeRegiseted)` iff any of the handles in the
    /// substitution `sigma` does not point to a registered type in the kernel's
    /// type-table.
    pub fn constant_at_constrained_type(
        constant: Handle,
        sigma: Vec<(Name, Handle)>,
    ) -> Result<Self, ErrorCode> {
        let mut tau = constant_type(&constant).ok_or(Err(ErrorCode::NoSuchConstantRegistered))?;

        for (domain, range) in sigma.iter().cloned() {
            tau = tau.substitute(
                domain,
                _type(&range).ok_or(Err(ErrorCode::NoSuchTypeRegistered))?,
            );
        }

        let handle = register_type(tau)?;

        Ok(Term::Constant {
            handle: constant,
            _type: Some(handle),
        })
    }

    /// Constructs a new lambda-abstraction lambda-term from a name, a handle to
    /// a type, `tau`, and a handle to an expression representing the body of
    /// the function, `body`.
    ///
    /// Returns `Err(ErrorCode::NoSuchTypeRegistered)` iff `tau` does not point
    /// to a registered type in the kernel's type-table.  Returns
    /// `Err(ErrorCode::NoSuchTermRegistered)` iff `body` does not point to a
    /// registered term in the kernel's term-table.
    pub fn lambda<T>(name: T, tau: Handle, body: Handle) -> Result<Self, ErrorCode>
    where
        T: Into<Name>,
    {
        if !is_type_registered(&tau) {
            return Err(ErrorCode::NoSuchTypeRegistered);
        }

        if !is_term_registered(&body) {
            return Err(ErrorCode::NoSuchTermRegistered);
        }

        Ok(Term::Lambda {
            name: name.into(),
            _type: tau,
            body,
        })
    }

    /// Returns the set of *free-variables* of a term, that is, all variables
    /// appearing in the term that are not bound by a lambda-abstraction.
    ///
    /// Will **panic** if the term is malformed and contains a dangling handle.
    pub fn fv(&self) -> Vec<Name> {
        match self {
            Term::Variable { name, .. } => vec![name],
            Term::Constant { .. } => Vec::new(),
            Term::Application { left, right } => {
                let left = term(left).expect(DANGLING_HANDLE_ERROR);
                let right = term(right).expect(DANGLING_HANDLE_ERROR);

                let mut fv = left.fv();
                fv.append(&mut right.fv());

                fv.sort();
                fv.dedup();

                fv
            }
            Term::Lambda { name, body, .. } => {
                let body = term(body).expect(DANGLING_HANDLE_ERROR);

                let mut fv = body.fv();

                fv.sort();
                fv.dedup();

                fv.iter().filter(|i| *i != name).collect()
            }
        }
    }

    /// Swaps (permutes) the names `a` and `b` in the term.  Note that this
    /// function may allocate new terms in the type-table as a result of this
    /// action.  This is used to define alpha-equivalence between terms and the
    /// capture-avoiding substitution action.
    ///
    /// Will **panic** if the term is malformed and contains dangling handles.
    fn swap<T>(&self, a: T, b: T) -> Self
    where
        T: Into<Name> + Clone,
    {
        match self {
            Term::Variable { name, _type } => {
                if name == a.into() {
                    Term::variable(b, _type.clone()).expect(PRIMITIVE_CONSTRUCTION_ERROR)
                } else if name == b.into() {
                    Term::variable(a, _type.clone()).expect(PRIMITIVE_CONSTRUCTION_ERROR)
                } else {
                    (*self).clone()
                }
            }
            Term::Constant { .. } => (*self).clone(),
            Term::Application { left, right } => {
                let left = register_term(
                    term(left)
                        .expect(DANGLING_HANDLE_ERROR)
                        .swap(a.clone(), b.clone()),
                )
                .expect(PRIMITIVE_CONSTRUCTION_ERROR);
                let right = register_term(term(right).expect(DANGLING_HANDLE_ERROR).swap(a, b))
                    .expect(PRIMITIVE_CONSTRUCTION_ERROR);

                Term::Application { left, right }
            }
            Term::Lambda { name, _type, body } => {
                let name = if name == a.into() {
                    b.into()
                } else if name == b.into() {
                    a.into()
                } else {
                    name
                };

                let body = register_term(
                    term(body)
                        .expect(DANGLING_HANDLE_ERROR)
                        .swap(a.clone(), b.clone()),
                )
                .expect(PRIMITIVE_CONSTRUCTION_ERROR);

                Term::Lambda {
                    name,
                    _type: *_type,
                    body,
                }
            }
        }
    }

    /// Returns `true` iff the two terms are alpha-equivalent, that is equal
    /// up-to a permutative renaming of their bound names.
    ///
    /// Will **panic** if the term is malformed, and contains dangling handles.
    pub fn is_alpha_equivalent(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Term::Variable {
                    name: name0,
                    _type: _type0,
                },
                Term::Variable {
                    name: name1,
                    _type: _type1,
                },
            ) => name0 == name1 && _type0 == _type1,
            (
                Term::Constant {
                    handle: handle0,
                    _type: _type0,
                },
                Term::Constant {
                    handle: handle1,
                    _type: _type1,
                },
            ) => handle0 == handle1 && _type0 == _type1,
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
                let left0 = term(left0).expect(DANGLING_HANDLE_ERROR);
                let right0 = term(right0).expect(DANGLING_HANDLE_ERROR);
                let left1 = term(left1).expect(DANGLING_HANDLE_ERROR);
                let right1 = term(right1).expect(DANGLING_HANDLE_ERROR);

                left0.is_alpha_equivalent(&left1) && right0.is_alpha_equivalent(&right1)
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
                    let body0 = term(body0).expect(DANGLING_HANDLE_ERROR);
                    let body1 = term(body1).expect(DANGLING_HANDLE_ERROR);

                    body0.is_alpha_equivalent(&body1) && _type0 == _type1
                } else {
                    let body1 = term(body1).expect(DANGLING_HANDLE_ERROR);

                    if body1.fv().contains(name0) {
                        false
                    } else {
                        let body0 = term(body0).expect(DANGLING_HANDLE_ERROR);

                        body1.swap(name0, name1).is_alpha_equivalent(&body0) && _type0 == _type1
                    }
                }
            }
            _otherwise => false,
        }
    }

    /// Returns `true` iff the term is well-formed, in the sense that it
    /// contains no dangling handles to other terms, and all types mentioned in
    /// the term are also well-formed.
    ///
    /// Note that is all "registration" functions for terms, types, constants,
    /// and so on, preserve the invariant that they only ever allow a kernel
    /// object to be registered if it is well-formed, then this check only needs
    /// to be shallow: only the handles actually appearing at the very "top" of
    /// the term need to be checked to ensure that the entire term is
    /// well-formed.
    pub fn is_well_formed(&self) -> bool {
        match self {
            Term::Variable { _type, .. } => is_type_registered(_type),
            Term::Constant { handle, _type } => match _type {
                None => is_constant_registered(handle),
                Some(_type) => is_type_registered(_type) && is_constant_registered(handle),
            },
            Term::Application { left, right } => {
                is_term_registered(left) && is_term_registered(right)
            }
            Term::Lambda { name, _type, body } => {
                is_type_registered(_type) && is_term_registered(body)
            }
        }
    }
}

////////////////////////////////////////////////////////////////////////////////
// The term-table.
////////////////////////////////////////////////////////////////////////////////

lazy_static! {
    // TODO: consider adding common terms, here, as pre-allocated conveniences.
    static ref TERM_TABLE: Mutex<HashMap<Handle, Term>> = Mutex::new(HashMap::new());
}

/// Registers a new term in the term-table, returning the handle to the term if
/// it is already registered (up-to alpha-equivalence), otherwise inventing a
/// new handle and returning that.  Note that the complexity of registering a
/// term in the table therefore becomes linear in the number of terms currently
/// registered in the table, but enforces maximal *sharing* in the sense that
/// terms are not registered unnecessarily and every handle is therefore a
/// unique reference to the equivalence class of alpha-equivalent terms. As a
/// result, equality checking on terms can be shallow, looking only at handles
/// without actually recursively fetching terms and types from the respective
/// tables and examining their structure.
///
/// Will **panic** if a lock on the term-table cannot be obtained.
pub fn register_term(trm: Term) -> Result<Handle, ErrorCode> {
    let mut table = TERM_TABLE.lock().expect(TABLE_LOCK_ERROR);

    if !trm.is_well_formed() {
        return Err(ErrorCode::TermNotWellformed);
    }

    for (handle, registered) in table.iter() {
        if registered.is_alpha_equivalent(&trm) {
            return Ok(*handle);
        }
    }

    let fresh = issue_handle();

    table.insert(fresh, trm);

    Ok(fresh)
}

/// Returns `Some(trm)` iff a term `trm` associated with the handle is
/// registered in the type-table (up-to alpha-equivalence).
///
/// Will **panic** if a lock on the term-table cannot be obtained.
pub fn term(handle: &Handle) -> Option<Term> {
    TERM_TABLE
        .lock()
        .expect(TABLE_LOCK_ERROR)
        .get(handle)
        .map(|t| t.clone())
}

/// Returns `true` iff a term (up-to alpha-equivalence) is associated with the
/// handle is registered in the term-table.
///
/// Will **panic** if a lock on the term-table cannot be obtained.
#[inline]
pub fn is_term_registered(handle: &Handle) -> bool {
    term(handle).is_some()
}

////////////////////////////////////////////////////////////////////////////////
// Back to the material...
////////////////////////////////////////////////////////////////////////////////

impl Term {
    pub fn infer_type(&self) -> Result<Type, ErrorCode> {
        match self {
            Term::Variable { _type, .. } => Ok(_type(_type).expect(DANGLING_HANDLE_ERROR)),
            Term::Constant { handle, _type, .. } => {
                if let Some(_type) = _type {
                    Ok(_type(_type).expect(DANGLING_HANDLE_ERROR))
                } else {
                    Ok(constant_type(handle).expect(DANGLING_HANDLE_ERROR))
                }
            }
            Term::Application { left, right } => {
                let left = term(left).expect(DANGLING_HANDLE_ERROR);
                let right = term(right).expect(DANGLING_HANDLE_ERROR);

                if let Some((dom, rng)) = left.infer_type()?.split_function() {
                    let dom = _type(dom).expect(DANGLING_HANDLE_ERROR);
                    let rng = _type(rng).expect(DANGLING_HANDLE_ERROR);

                    if dom == right.infer_type()? {
                        Ok(rng)
                    } else {
                        Err(ErrorCode::DomainTypeMismatch)
                    }
                } else {
                    Err(ErrorCode::NotAFunctionType)
                }
            }
            Term::Lambda { _type, body, .. } => {
                let _type = _type(_type).expect(DANGLING_HANDLE_ERROR);
                let body = term(body).expect(DANGLING_HANDLE_ERROR);

                Ok(Type::function(_type, body.infer_type()?))
            }
        }
    }

    /// Returns `Ok(true)` iff the type has `Prop` type.
    #[inline]
    pub fn is_proposition(&self) -> Result<bool, ErrorCode> {
        Ok(self.infer_type()?.is_prop())
    }

    pub fn application(left: Handle, right: Handle) -> Result<Self, ErrorCode> {
        let left_trm = term(&left).ok_or(Err(ErrorCode::NoSuchTermRegistered))?;
        let right_trm = term(&right).ok_or(Err(ErrorCode::NoSuchTermRegistered))?;

        let (dom, _rng) = left_trm.infer_type()?.split_function()?;
        let arg = right_trm.infer_type()?;

        let dom = _type(dom).expect(DANGLING_HANDLE_ERROR);

        if dom == arg {
            Ok(Term::Application { left, right })
        } else {
            Err(ErrorCode::DomainTypeMismatch)
        }
    }

    pub fn lambda<T>(name: T, _type: Handle, body: Handle) -> Result<Self, ErrorCode>
    where
        T: Into<String>,
    {
        if !is_type_registered(&_type) {
            return Err(ErrorCode::NoSuchTypeRegistered);
        }

        if !is_term_registered(&body) {
            return Err(ErrorCode::NoSuchTermRegistered);
        }

        Ok(Term::Lambda { name, _type, body })
    }

    pub fn type_substitution(&self, sigma: Vec<(Name, Handle)>) -> Self {
        match self {
            Term::Variable { name, _type } => unimplemented!(),
            Term::Constant { handle, _type } => unimplemented!(),
            Term::Application { left, right } => unimplemented!(),
            Term::Lambda { name, _type, body } => unimplemented!(),
        }
    }

    pub fn substitution(&self, sigma: Vec<(Name, Handle)>) -> Self {
        match self {
            Term::Variable { name, _type } => unimplemented!(),
            Term::Constant { handle, _type } => unimplemented!(),
            Term::Application { left, right } => unimplemented!(),
            Term::Lambda { name, _type, body } => unimplemented!(),
        }
    }
}
