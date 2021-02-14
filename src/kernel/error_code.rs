//! # Error codes
//!
//! In most LCF-style proof assistants, errors are signalled via exceptions.  We
//! cannot use exceptions in Supervisionary, so use error codes instead.  Note
//! that the contents of this file must also be mirror in prover-space, as it
//! forms part of the ABI contract between kernel and prover.
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

use std::convert::TryFrom;
use std::fmt::{Display, Error as DisplayError, Formatter};

/// Error codes, used for passing back information on why a kernel operation
/// failed to prover-space.  These codes are intra-convertible between the `i32`
/// type.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(i32)]
pub enum ErrorCode {
    /* ABI errors. */
    /// The operation completed successfully.
    Success,
    /// The type-signature of an ABI function was not as expected.
    SignatureFailure,
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
    NotAForall,
    NotADisjunction,
    /// A term passed to a function was expected to be a lambda-abstraction but
    /// it was not.
    NotALambda,
    /// A term passed to a function was expected to be an application but it was
    /// not.
    NotAnApplication,
    NotAnEquality,
    NotAnExists,
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

impl Display for ErrorCode {
    fn fmt(&self, f: &mut Formatter) -> Result<(), DisplayError> {
        match self {
            ErrorCode::Success => write!(f, "Success"),
            ErrorCode::SignatureFailure => write!(f, "SignatureFailure"),
            ErrorCode::NoSuchFunction => write!(f, "NoSuchFunction"),
            ErrorCode::NoSuchConstantRegistered => write!(f, "NoSuchConstantRegistered"),
            ErrorCode::NoSuchTermRegistered => write!(f, "NoSuchTermRegistered"),
            ErrorCode::NoSuchTheoremRegistered => write!(f, "NoSuchTheoremRegistered"),
            ErrorCode::NoSuchTypeFormerRegistered => write!(f, "NoSuchTypeFormerRegistered"),
            ErrorCode::MismatchedArity => write!(f, "MismatchedArity"),
            ErrorCode::DomainTypeMismatch => write!(f, "DomainTypeMismatch"),
            ErrorCode::NoSuchTypeRegistered => write!(f, "NoSuchTypeRegistered"),
            ErrorCode::NotAFunctionType => write!(f, "NotAFunctionType"),
            ErrorCode::NotATypeCombination => write!(f, "NotATypeCombination"),
            ErrorCode::NotATypeVariable => write!(f, "NotATypeVariable"),
            ErrorCode::TypeNotWellformed => write!(f, "TypeNotWellformed"),
            ErrorCode::NotAConjunction => write!(f, "NotAConjunction"),
            ErrorCode::NotAConstant => write!(f, "NotAConstant"),
            ErrorCode::NotAForall => write!(f, "NotAForall"),
            ErrorCode::NotADisjunction => write!(f, "NotADisjunction"),
            ErrorCode::NotALambda => write!(f, "NotALambda"),
            ErrorCode::NotAnApplication => write!(f, "NotAnApplication"),
            ErrorCode::NotAnEquality => write!(f, "NotAnEquality"),
            ErrorCode::NotAnExists => write!(f, "NotAnExists"),
            ErrorCode::NotAnImplication => write!(f, "NotAnImplication"),
            ErrorCode::NotANegation => write!(f, "NotANegation"),
            ErrorCode::NotAProposition => write!(f, "NotAProposition"),
            ErrorCode::NotAVariable => write!(f, "NotAVariable"),
            ErrorCode::TermNotWellformed => write!(f, "TermNotWellformed"),
            ErrorCode::ShapeMismatch => write!(f, "ShapeMismatch"),
            ErrorCode::TheoremNotWellformed => write!(f, "TheoremNotWellformed"),
        }
    }
}
