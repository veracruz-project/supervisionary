//! # Error codes
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
    convert::TryFrom,
    fmt::{Display, Error as DisplayError, Formatter},
};

////////////////////////////////////////////////////////////////////////////////
// Error codes.
////////////////////////////////////////////////////////////////////////////////

/// Error codes, used for passing back information on why a kernel operation
/// failed to prover-space.  These codes are intra-convertible between the `i32`
/// type.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ErrorCode {
    /* ABI errors. */
    /// The WASM guest program tried to call a host function that does not
    /// exist.
    NoSuchFunction,
    /* Dangling objects. */
    /// A handle was supplied that did not reference a registered constant.
    NoSuchConstantRegistered,
    /// A handle was supplied that did not reference a registered term.
    NoSuchTermRegistered,
    /// A handle was supplied that did not reference a registered theorem.
    NoSuchTheoremRegistered,
    /// A handle was supplied that did not reference a registered type-former.
    NoSuchTypeFormerRegistered,
    /* Type-former related errors. */
    /// A type-former was applied to the wrong number of arguments.
    MismatchedArity,
    /* -- Type related errors. */
    /// A term with functional type was applied to an argument that had a
    /// different type to the domain type of the function.
    DomainTypeMismatch,
    /// A handle was supplied that did not reference a registered type.
    NoSuchTypeRegistered,
    /// A type was expected to be a functional type, but it was not.
    NotAFunctionType,
    /// A type was expected to be a type-combination, but it was not.
    NotATypeCombination,
    /// A type was expected to be a type-variable, but it was not.
    NotATypeVariable,
    /// A type passed to a function as an argument was not well-formed.
    TypeNotWellformed,
    /* -- Constant related errors. */
    /* -- Term related errors. */
    NotAConjunction,
    /// A term passed to a function was expected to be a constant but it was
    /// not.
    NotAConstant,
    /// A term passed to a function was expected to be a universal quantifier
    /// but it was not.
    NotAForall,
    /// A term passed to a function was expected to be a disjunction but it was
    /// not.
    NotADisjunction,
    /// A term passed to a function was expected to be a lambda-abstraction but
    /// it was not.
    NotALambda,
    /// A term passed to a function was expected to be an application but it was
    /// not.
    NotAnApplication,
    /// A term passed to a function was expected to be an equality but it was
    /// not.
    NotAnEquality,
    /// A term passed to a function was expected to be an existential quantifier
    /// but it was not.
    NotAnExists,
    /// A term passed to a function was expected to be an implication but it was
    /// not.
    NotAnImplication,
    /// A term passed to a function was expected to be a negation but it was
    /// not.
    NotANegation,
    /// A term passed to a function as an argument did not have propositional
    /// type.
    NotAProposition,
    /// A term passed to a function was expected to be a variable but it was
    /// not.
    NotAVariable,
    /// A term passed to a function as an argument was not well-formed.
    TermNotWellformed,
    /* -- Theorem related errors. */
    /// An inference rule expected its hypotheses to be in a certain shape, but
    /// they were not.
    ShapeMismatch,
    /// A theorem passed to a function as an argument was not well-formed.
    TheoremNotWellformed,
}

////////////////////////////////////////////////////////////////////////////////
// Trait implementations.
////////////////////////////////////////////////////////////////////////////////

/// Conversion into an `i32` type for ABI transport.
impl Into<i32> for ErrorCode {
    fn into(self) -> i32 {
        match self {
            ErrorCode::NoSuchFunction => 1,
            ErrorCode::NoSuchConstantRegistered => 2,
            ErrorCode::NoSuchTermRegistered => 3,
            ErrorCode::NoSuchTheoremRegistered => 4,
            ErrorCode::NoSuchTypeFormerRegistered => 5,
            ErrorCode::MismatchedArity => 6,
            ErrorCode::DomainTypeMismatch => 7,
            ErrorCode::NoSuchTypeRegistered => 8,
            ErrorCode::NotAFunctionType => 9,
            ErrorCode::NotATypeCombination => 10,
            ErrorCode::NotATypeVariable => 11,
            ErrorCode::TypeNotWellformed => 12,
            ErrorCode::NotAConjunction => 13,
            ErrorCode::NotAConstant => 14,
            ErrorCode::NotAForall => 15,
            ErrorCode::NotADisjunction => 16,
            ErrorCode::NotALambda => 17,
            ErrorCode::NotAnApplication => 18,
            ErrorCode::NotAnEquality => 19,
            ErrorCode::NotAnExists => 20,
            ErrorCode::NotAnImplication => 21,
            ErrorCode::NotANegation => 22,
            ErrorCode::NotAProposition => 23,
            ErrorCode::NotAVariable => 24,
            ErrorCode::TermNotWellformed => 25,
            ErrorCode::ShapeMismatch => 26,
            ErrorCode::TheoremNotWellformed => 27,
        }
    }
}

impl TryFrom<i32> for ErrorCode {
    type Error = ();

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(ErrorCode::NoSuchFunction),
            2 => Ok(ErrorCode::NoSuchConstantRegistered),
            3 => Ok(ErrorCode::NoSuchTermRegistered),
            4 => Ok(ErrorCode::NoSuchTheoremRegistered),
            5 => Ok(ErrorCode::NoSuchTypeFormerRegistered),
            6 => Ok(ErrorCode::MismatchedArity),
            7 => Ok(ErrorCode::DomainTypeMismatch),
            8 => Ok(ErrorCode::NoSuchTypeRegistered),
            9 => Ok(ErrorCode::NotAFunctionType),
            10 => Ok(ErrorCode::NotATypeCombination),
            11 => Ok(ErrorCode::NotATypeVariable),
            12 => Ok(ErrorCode::TypeNotWellformed),
            13 => Ok(ErrorCode::NotAConjunction),
            14 => Ok(ErrorCode::NotAConstant),
            15 => Ok(ErrorCode::NotAForall),
            16 => Ok(ErrorCode::NotADisjunction),
            17 => Ok(ErrorCode::NotALambda),
            18 => Ok(ErrorCode::NotAnApplication),
            19 => Ok(ErrorCode::NotAnEquality),
            20 => Ok(ErrorCode::NotAnExists),
            21 => Ok(ErrorCode::NotAnImplication),
            22 => Ok(ErrorCode::NotANegation),
            23 => Ok(ErrorCode::NotAProposition),
            24 => Ok(ErrorCode::NotAVariable),
            25 => Ok(ErrorCode::TermNotWellformed),
            26 => Ok(ErrorCode::ShapeMismatch),
            27 => Ok(ErrorCode::TheoremNotWellformed),
            _otherwise => Err(()),
        }
    }
}
