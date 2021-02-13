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

/// Error codes, used for passing back information on why a kernel operation
/// failed to prover-space.  These codes are intra-convertible between the `i32`
/// type.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ErrorCode {
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
