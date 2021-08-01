//! # WASMI host interface
//!
//! This binds the kernel code, proper, to the guest WASM program-facing ABI
//! interface, routing host-calls as appropriate.
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
    borrow::Borrow,
    cell::RefCell,
    fmt::{Debug, Display, Error as DisplayError, Formatter},
    mem::size_of,
};

use byteorder::{ByteOrder, LittleEndian};
use wasmi::{
    Error as WasmiError, Externals, FuncInstance, FuncRef, HostError,
    MemoryInstance, ModuleImportResolver, RuntimeArgs, RuntimeValue, Signature,
    Trap, TrapKind, ValueType,
};

use crate::kernel::{
    error_code::ErrorCode as KernelErrorCode,
    handle::{tags, Handle},
    name::Name,
    runtime_state::RuntimeState as KernelRuntimeState,
};

////////////////////////////////////////////////////////////////////////////////
// Semantic types for the WASM ABI.
////////////////////////////////////////////////////////////////////////////////

/// Type-synonyms for declaratively describing the intended purpose of WASM
/// types passed across the ABI boundary.
mod semantic_types {
    /// A WASM address, used for reading-from and writing-to the guest WASM
    /// program heap, assuming the `wasm32-abi`.
    pub type Pointer = u32;
    /// An arity of a type-former.
    pub type Arity = u64;
    /// A handle to a kernel object.
    pub type Handle = u64;
    /// A name of a variable (e.g. a lambda-abstracted variable, or
    /// type-variable).
    pub type Name = u64;
    /// A size of a buffer, or other heap-allocated structure, used for
    /// reading-from and writing-to the guest WASM program heap, assuming the
    /// `wasm32-abi`.
    pub type Size = u64;
}

/// A type capturing semantic types of the ABI, more descriptive than the base
/// types of WASM.  Note that the constructors of this type are intended to shadow
/// the type-synyonyms defined in the `semantic_types` module.
enum AbiType {
    /// A handle pointing-to a kernel object.
    Handle,
    /// A name (e.g. of a lambda-abstracted variable, or similar).
    Name,
    /// An arity for a type-former.
    Arity,
    /// A pointer into the host WASM program's heap.
    Pointer,
    /// A size (or length) of an object appearing in the WASM program's heap.
    Size,
    /// A Boolean value.
    Boolean,
    /// An error code returned from an ABI function.
    ErrorCode,
}

impl AbiType {
    /// Returns `true` iff the current `AbiType` is implemented by the WASM
    /// value type, `tau`.
    fn implemented_by(&self, tau: &ValueType) -> bool {
        match self {
            AbiType::Boolean => tau == &ValueType::I32,
            AbiType::Handle => tau == &ValueType::I64,
            AbiType::Arity => tau == &ValueType::I64,
            AbiType::Name => tau == &ValueType::I64,
            AbiType::Pointer => tau == &ValueType::I32,
            AbiType::Size => tau == &ValueType::I64,
            AbiType::ErrorCode => tau == &ValueType::I32,
        }
    }
}

////////////////////////////////////////////////////////////////////////////////
// ABI: host-call names and numbers.
////////////////////////////////////////////////////////////////////////////////

/* Type-former related calls. */

/// The name of the `TypeFormer.Resolve` ABI call.
pub const ABI_TYPE_FORMER_RESOLVE_NAME: &str = "__type_former_resolve";
/// The name of the `TypeFormer.IsRegistered` ABI call.
pub const ABI_TYPE_FORMER_IS_REGISTERED_NAME: &str =
    "__type_former_is_registered";
/// The name of the `TypeFormer.Register` ABI call.
pub const ABI_TYPE_FORMER_REGISTER_NAME: &str = "__type_former_register";

/// The host-call number of the `TypeFormer.Resolve` ABI call.
pub const ABI_TYPE_FORMER_RESOLVE_INDEX: usize = 0;
/// The host-call number of the `TypeFormer.IsRegistered` ABI call.
pub const ABI_TYPE_FORMER_IS_REGISTERED_INDEX: usize = 1;
/// The host-call number of the `TypeFormer.Register` ABI call.
pub const ABI_TYPE_FORMER_REGISTER_INDEX: usize = 2;

/* Type-related calls. */

/// The name of the `Type.IsRegistered` ABI call.
pub const ABI_TYPE_IS_REGISTERED_NAME: &str = "__type_is_registered";
/// The name of the `Type.Register.Variable` ABI call.
pub const ABI_TYPE_REGISTER_VARIABLE_NAME: &str = "__type_register_variable";
/// The name of the `Type.Register.Combination` ABI call.
pub const ABI_TYPE_REGISTER_COMBINATION_NAME: &str =
    "__type_register_combination_name";
/// The name of the `Type.Register.Function` ABI call.
pub const ABI_TYPE_REGISTER_FUNCTION_NAME: &str =
    "__type_register_function_name";

/// The name of the `Type.Split.Variable` ABI call.
pub const ABI_TYPE_SPLIT_VARIABLE_NAME: &str = "__type_split_variable_name";
/// The name of the `Type.Split.Combination` ABI call.
pub const ABI_TYPE_SPLIT_COMBINATION_NAME: &str =
    "__type_split_combination_name";
/// The name of the `Type.Split.Function` ABI call.
pub const ABI_TYPE_SPLIT_FUNCTION_NAME: &str = "__type_split_function_name";

/// The name of the `Type.Test.Variable` ABI call.
pub const ABI_TYPE_TEST_VARIABLE_NAME: &str = "__type_test_variable";
/// The name of the `Type.Test.Combination` ABI call.
pub const ABI_TYPE_TEST_COMBINATION_NAME: &str = "__type_test_combination";
/// The name of the `Type.Test.Function` ABI call.
pub const ABI_TYPE_TEST_FUNCTION_NAME: &str = "__type_test_function";

/// The name of the `Type.Variables` ABI call.
pub const ABI_TYPE_FTV_NAME: &str = "__type_variables";
/// The name of the `Type.Substitute` ABI call.
pub const ABI_TYPE_SUBSTITUTE_NAME: &str = "__type_substitute";

/// The host-call number of the `Type.IsRegistered` ABI call.
pub const ABI_TYPE_IS_REGISTERED_INDEX: usize = 3;
/// The host-call number of the `Type.Register.Variable` ABI call.
pub const ABI_TYPE_REGISTER_VARIABLE_INDEX: usize = 4;
/// The host-call number of the `Type.Register.Combination` ABI call.
pub const ABI_TYPE_REGISTER_COMBINATION_INDEX: usize = 5;
/// The host-call number of the `Type.Register.Function` ABI call.
pub const ABI_TYPE_REGISTER_FUNCTION_INDEX: usize = 6;

/// The host-call number of the `Type.Split.Variable` ABI call.
pub const ABI_TYPE_SPLIT_VARIABLE_INDEX: usize = 7;
/// The host-call number of the `Type.Split.Combination` ABI call.
pub const ABI_TYPE_SPLIT_COMBINATION_INDEX: usize = 8;
/// The host-call number of the `Type.Split.Function` ABI call.
pub const ABI_TYPE_SPLIT_FUNCTION_INDEX: usize = 9;

/// The host-call number of the `Type.Test.Variable` ABI call.
pub const ABI_TYPE_TEST_VARIABLE_INDEX: usize = 10;
/// The host-call number of the `Type.Test.Combination` ABI call.
pub const ABI_TYPE_TEST_COMBINATION_INDEX: usize = 11;
/// The host-call number of the `Type.Test.Function` ABI call.
pub const ABI_TYPE_TEST_FUNCTION_INDEX: usize = 12;

/// The host-call number of the `Type.Variables` ABI call.
pub const ABI_TYPE_VARIABLES_INDEX: usize = 13;
/// The host-call number of the `Type.Substitute` ABI call.
pub const ABI_TYPE_SUBSTITUTE_INDEX: usize = 14;

/* Constant-related calls. */

/// The name of the `Constant.Resolve` ABI call.
pub const ABI_CONSTANT_RESOLVE_NAME: &str = "__constant_resolve";
/// The name of the `Constant.IsRegistered` ABI call.
pub const ABI_CONSTANT_IS_REGISTERED_NAME: &str = "__constant_is_registered";
/// The name of the `Constant.Register` ABI call.
pub const ABI_CONSTANT_REGISTER_NAME: &str = "__constant_register";

/// The host-call number of the `Constant.Resolve` ABI call.
pub const ABI_CONSTANT_RESOLVE_INDEX: usize = 15;
/// The host-call number of the `Constant.IsRegistered` ABI call.
pub const ABI_CONSTANT_IS_REGISTERED_INDEX: usize = 16;
/// The host-call number of the `Constant.Register` ABI call.
pub const ABI_CONSTANT_REGISTER_INDEX: usize = 17;

/* Term-related calls. */

/// The name of the `Term.Register.Variable` ABI call.
pub const ABI_TERM_REGISTER_VARIABLE_NAME: &str = "__term_register_variable";
/// The name of the `Term.Register.Constant` ABI call.
pub const ABI_TERM_REGISTER_CONSTANT_NAME: &str = "__term_register_constant";
/// The name of the `Term.Register.Application` ABI call.
pub const ABI_TERM_REGISTER_APPLICATION_NAME: &str =
    "__term_register_application";
/// The name of the `Term.Register.Lambda` ABI call.
pub const ABI_TERM_REGISTER_LAMBDA_NAME: &str = "__term_register_lambda";
/// The name of the `Term.Register.Negation` ABI call.
pub const ABI_TERM_REGISTER_NEGATION_NAME: &str = "__term_register_negation";
/// The name of the `Term.Register.Conjunction` ABI call.
pub const ABI_TERM_REGISTER_CONJUNCTION_NAME: &str =
    "__term_register_conjunction";
/// The name of the `Term.Register.Disjunction` ABI call.
pub const ABI_TERM_REGISTER_DISJUNCTION_NAME: &str =
    "__term_register_disjunction";
/// The name of the `Term.Register.Implication` ABI call.
pub const ABI_TERM_REGISTER_IMPLICATION_NAME: &str =
    "__term_register_implication";
/// The name of the `Term.Register.Equality` ABI call.
pub const ABI_TERM_REGISTER_EQUALITY_NAME: &str = "__term_register_equality";
/// The name of the `Term.Register.Forall` ABI call.
pub const ABI_TERM_REGISTER_FORALL_NAME: &str = "__term_register_forall";
/// The name of the `Term.Register.Exists` ABI call.
pub const ABI_TERM_REGISTER_EXISTS_NAME: &str = "__term_register_exists";

/// The name of the `Term.Split.Variable` ABI call.
pub const ABI_TERM_SPLIT_VARIABLE_NAME: &str = "__term_split_variable";
/// The name of the `Term.Split.Constant` ABI call.
pub const ABI_TERM_SPLIT_CONSTANT_NAME: &str = "__term_split_constant";
/// The name of the `Term.Split.Application` ABI call.
pub const ABI_TERM_SPLIT_APPLICATION_NAME: &str = "__term_split_application";
/// The name of the `Term.Split.Lambda` ABI call.
pub const ABI_TERM_SPLIT_LAMBDA_NAME: &str = "__term_split_lambda";
/// The name of the `Term.Split.Negation` ABI call.
pub const ABI_TERM_SPLIT_NEGATION_NAME: &str = "__term_split_negation";
/// The name of the `Term.Split.Conjunction` ABI call.
pub const ABI_TERM_SPLIT_CONJUNCTION_NAME: &str = "__term_split_conjunction";
/// The name of the `Term.Split.Disjunction` ABI call.
pub const ABI_TERM_SPLIT_DISJUNCTION_NAME: &str = "__term_split_disjunction";
/// The name of the `Term.Split.Implication` ABI call.
pub const ABI_TERM_SPLIT_IMPLICATION_NAME: &str = "__term_split_implication";
/// The name of the `Term.Split.Equality` ABI call.
pub const ABI_TERM_SPLIT_EQUALITY_NAME: &str = "__term_split_equality";
/// The name of the `Term.Split.Forall` ABI call.
pub const ABI_TERM_SPLIT_FORALL_NAME: &str = "__term_split_forall";
/// The name of the `Term.Split.Exists` ABI call.
pub const ABI_TERM_SPLIT_EXISTS_NAME: &str = "__term_split_exists";

/// The name of the `Term.Test.Variable` ABI call.
pub const ABI_TERM_TEST_VARIABLE_NAME: &str = "__term_test_variable";
/// The name of the `Term.Test.Constant` ABI call.
pub const ABI_TERM_TEST_CONSTANT_NAME: &str = "__term_test_constant";
/// The name of the `Term.Test.Application` ABI call.
pub const ABI_TERM_TEST_APPLICATION_NAME: &str = "__term_test_application";
/// The name of the `Term.Test.Lambda` ABI call.
pub const ABI_TERM_TEST_LAMBDA_NAME: &str = "__term_test_lambda";
/// The name of the `Term.Test.Negation` ABI call.
pub const ABI_TERM_TEST_NEGATION_NAME: &str = "__term_test_negation";
/// The name of the `Term.Test.Conjunction` ABI call.
pub const ABI_TERM_TEST_CONJUNCTION_NAME: &str = "__term_test_conjunction";
/// The name of the `Term.Test.Disjunction` ABI call.
pub const ABI_TERM_TEST_DISJUNCTION_NAME: &str = "__term_test_disjunction";
/// The name of the `Term.Test.Implication` ABI call.
pub const ABI_TERM_TEST_IMPLICATION_NAME: &str = "__term_test_implication";
/// The name of the `Term.Test.Equality` ABI call.
pub const ABI_TERM_TEST_EQUALITY_NAME: &str = "__term_test_equality";
/// The name of the `Term.Test.Forall` ABI call.
pub const ABI_TERM_TEST_FORALL_NAME: &str = "__term_test_forall";
/// The name of the `Term.Test.Exists` ABI call.
pub const ABI_TERM_TEST_EXISTS_NAME: &str = "__term_test_exists";

/// The name of the `Term.FreeVariables` ABI call.
pub const ABI_TERM_FREE_VARIABLES_NAME: &str = "__term_free_variables";
/// The name of the `Term.Substitution` ABI call.
pub const ABI_TERM_SUBSTITUTION_NAME: &str = "__term_substitution";

/// The name of the `Term.Type.Variables` ABI call.
pub const ABI_TERM_TYPE_VARIABLES_NAME: &str = "__term_type_variables";
/// The name of the `Term.Type.Substitution` ABI call.
pub const ABI_TERM_TYPE_SUBSTITUTION_NAME: &str = "__term_type_substitution";
/// The name of the `Term.Type.Infer` ABI call.
pub const ABI_TERM_TYPE_INFER_NAME: &str = "__term_type_infer";
/// The name of the `Term.Type.IsProposition` ABI call.
pub const ABI_TERM_TYPE_IS_PROPOSITION_NAME: &str =
    "__term_type_is_proposition";

/// The host-call number of the `Term.Register.Variable` ABI call.
pub const ABI_TERM_REGISTER_VARIABLE_INDEX: usize = 18;
/// The host-call number of the `Term.Register.Constant` ABI call.
pub const ABI_TERM_REGISTER_CONSTANT_INDEX: usize = 19;
/// The host-call number of the `Term.Register.Application` ABI call.
pub const ABI_TERM_REGISTER_APPLICATION_INDEX: usize = 20;
/// The host-call number of the `Term.Register.Lambda` ABI call.
pub const ABI_TERM_REGISTER_LAMBDA_INDEX: usize = 21;
/// The host-call number of the `Term.Register.Negation` ABI call.
pub const ABI_TERM_REGISTER_NEGATION_INDEX: usize = 22;
/// The host-call number of the `Term.Register.Conjunction` ABI call.
pub const ABI_TERM_REGISTER_CONJUNCTION_INDEX: usize = 23;
/// The host-call number of the `Term.Register.Disjunction` ABI call.
pub const ABI_TERM_REGISTER_DISJUNCTION_INDEX: usize = 24;
/// The host-call number of the `Term.Register.Implication` ABI call.
pub const ABI_TERM_REGISTER_IMPLICATION_INDEX: usize = 25;
/// The host-call number of the `Term.Register.Equality` ABI call.
pub const ABI_TERM_REGISTER_EQUALITY_INDEX: usize = 26;
/// The host-call number of the `Term.Register.Forall` ABI call.
pub const ABI_TERM_REGISTER_FORALL_INDEX: usize = 27;
/// The host-call number of the `Term.Register.Exists` ABI call.
pub const ABI_TERM_REGISTER_EXISTS_INDEX: usize = 28;

/// The host-call number of the `Term.Split.Variable` ABI call.
pub const ABI_TERM_SPLIT_VARIABLE_INDEX: usize = 29;
/// The host-call number of the `Term.Split.Constant` ABI call.
pub const ABI_TERM_SPLIT_CONSTANT_INDEX: usize = 30;
/// The host-call number of the `Term.Split.Application` ABI call.
pub const ABI_TERM_SPLIT_APPLICATION_INDEX: usize = 31;
/// The host-call number of the `Term.Split.Lambda` ABI call.
pub const ABI_TERM_SPLIT_LAMBDA_INDEX: usize = 32;
/// The host-call number of the `Term.Split.Negation` ABI call.
pub const ABI_TERM_SPLIT_NEGATION_INDEX: usize = 33;
/// The host-call number of the `Term.Split.Conjunction` ABI call.
pub const ABI_TERM_SPLIT_CONJUNCTION_INDEX: usize = 34;
/// The host-call number of the `Term.Split.Disjunction` ABI call.
pub const ABI_TERM_SPLIT_DISJUNCTION_INDEX: usize = 35;
/// The host-call number of the `Term.Split.Implication` ABI call.
pub const ABI_TERM_SPLIT_IMPLICATION_INDEX: usize = 36;
/// The host-call number of the `Term.Split.Equality` ABI call.
pub const ABI_TERM_SPLIT_EQUALITY_INDEX: usize = 37;
/// The host-call number of the `Term.Split.Forall` ABI call.
pub const ABI_TERM_SPLIT_FORALL_INDEX: usize = 38;
/// The host-call number of the `Term.Split.Exists` ABI call.
pub const ABI_TERM_SPLIT_EXISTS_INDEX: usize = 39;

/// The host-call number of the `Term.Test.Variable` ABI call.
pub const ABI_TERM_TEST_VARIABLE_INDEX: usize = 40;
/// The host-call number of the `Term.Test.Constant` ABI call.
pub const ABI_TERM_TEST_CONSTANT_INDEX: usize = 41;
/// The host-call number of the `Term.Test.Application` ABI call.
pub const ABI_TERM_TEST_APPLICATION_INDEX: usize = 42;
/// The host-call number of the `Term.Test.Lambda` ABI call.
pub const ABI_TERM_TEST_LAMBDA_INDEX: usize = 43;
/// The host-call number of the `Term.Test.Negation` ABI call.
pub const ABI_TERM_TEST_NEGATION_INDEX: usize = 44;
/// The host-call number of the `Term.Test.Conjunction` ABI call.
pub const ABI_TERM_TEST_CONJUNCTION_INDEX: usize = 45;
/// The host-call number of the `Term.Test.Disjunction` ABI call.
pub const ABI_TERM_TEST_DISJUNCTION_INDEX: usize = 46;
/// The host-call number of the `Term.Test.Implication` ABI call.
pub const ABI_TERM_TEST_IMPLICATION_INDEX: usize = 47;
/// The host-call number of the `Term.Test.Equality` ABI call.
pub const ABI_TERM_TEST_EQUALITY_INDEX: usize = 48;
/// The host-call number of the `Term.Test.Forall` ABI call.
pub const ABI_TERM_TEST_FORALL_INDEX: usize = 49;
/// The host-call number of the `Term.Test.Exists` ABI call.
pub const ABI_TERM_TEST_EXISTS_INDEX: usize = 50;

/// The host-call number of the `Term.FreeVariables` ABI call.
pub const ABI_TERM_FREE_VARIABLES_INDEX: usize = 51;
/// The host-call number of the `Term.Substitution` ABI call.
pub const ABI_TERM_SUBSTITUTION_INDEX: usize = 52;

/// The host-call number of the `Term.Type.Variables` ABI call.
pub const ABI_TERM_TYPE_VARIABLES_INDEX: usize = 53;
/// The host-call number of the `Term.Type.Substitution` ABI call.
pub const ABI_TERM_TYPE_SUBSTITUTION_INDEX: usize = 54;
/// The host-call number of the `Term.Type.Infer` ABI call.
pub const ABI_TERM_TYPE_INFER_INDEX: usize = 55;
/// The host-call number of the `Term.Type.IsProposition` ABI call.
pub const ABI_TERM_TYPE_IS_PROPOSITION_INDEX: usize = 56;

/* Theorem-related calls. */

/// The name of the `Theorem.IsRegistered` ABI call.
pub const ABI_THEOREM_IS_REGISTERED_NAME: &str = "__theorem_is_registered";

/// The name of the `Theorem.Register.Assumption` ABI call.
pub const ABI_THEOREM_REGISTER_ASSUMPTION_NAME: &str =
    "__theorem_register_assumption";

/// The name of the `Theorem.Register.Reflexivity` ABI call.
pub const ABI_THEOREM_REGISTER_REFLEXIVITY_NAME: &str =
    "__theorem_register_reflexivity";
/// The name of the `Theorem.Register.Symmetry` ABI call.
pub const ABI_THEOREM_REGISTER_SYMMETRY_NAME: &str =
    "__theorem_register_symmetry";
/// The name of the `Theorem.Register.Transitivity` ABI call.
pub const ABI_THEOREM_REGISTER_TRANSITIVITY_NAME: &str =
    "__theorem_register_transitivity";
/// The name of the `Theorem.Register.Beta` ABI call.
pub const ABI_THEOREM_REGISTER_BETA_NAME: &str = "__theorem_register_beta";
/// The name of the `Theorem.Register.Eta` ABI call.
pub const ABI_THEOREM_REGISTER_ETA_NAME: &str = "__theorem_register_eta";
/// The name of the `Theorem.Register.Application` ABI call.
pub const ABI_THEOREM_REGISTER_APPLICATION_NAME: &str =
    "__theorem_register_application";
/// The name of the `Theorem.Register.Lambda` ABI call.
pub const ABI_THEOREM_REGISTER_LAMBDA_NAME: &str = "__theorem_register_lambda";

/// The name of the `Theorem.Register.Substitution` ABI call.
pub const ABI_THEOREM_REGISTER_SUBSTITUTION_NAME: &str =
    "__theorem_register_substitution";
/// The name of the `Theorem.Register.TypeSubstitution` ABI call.
pub const ABI_THEOREM_REGISTER_TYPE_SUBSTITUTION_NAME: &str =
    "__theorem_register_type_substitution";

/// The name of the `Theorem.Register.TruthIntroduction` ABI call.
pub const ABI_THEOREM_REGISTER_TRUTH_INTRODUCTION_NAME: &str =
    "__theorem_register_truth_introduction";
/// The name of the `Theorem.Register.FalsityElimination` ABI call.
pub const ABI_THEOREM_REGISTER_FALSITY_ELIMINATION_NAME: &str =
    "__theorem_register_falsity_elimination";

/// The name of the `Theorem.Register.ConjunctionIntroduction` ABI call.
pub const ABI_THEOREM_REGISTER_CONJUNCTION_INTRODUCTION_NAME: &str =
    "__theorem_register_conjunction_introduction";
/// The name of the `Theorem.Register.ConjunctionLeftElimination` ABI call.
pub const ABI_THEOREM_REGISTER_CONJUNCTION_LEFT_ELIMINATION_NAME: &str =
    "__theorem_register_conjunction_left_elimination";
/// The name of the `Theorem.Register.ConjunctionRightElimination` ABI call.
pub const ABI_THEOREM_REGISTER_CONJUNCTION_RIGHT_ELIMINATION_NAME: &str =
    "__theorem_register_conjunction_right_elimination";

/// The name of the `Theorem.Register.DisjunctionElimination` ABI call.
pub const ABI_THEOREM_REGISTER_DISJUNCTION_ELIMINATION_NAME: &str =
    "__theorem_register_disjunction_elimination";
/// The name of the `Theorem.Register.DisjunctionLeftIntroduction` ABI call.
pub const ABI_THEOREM_REGISTER_DISJUNCTION_LEFT_INTRODUCTION_NAME: &str =
    "__theorem_register_disjunction_left_introduction";
/// The name of the `Theorem.Register.DisjunctionRightIntroduction` ABI call.
pub const ABI_THEOREM_REGISTER_DISJUNCTION_RIGHT_INTRODUCTION_NAME: &str =
    "__theorem_register_disjunction_right_introduction";

/// The name of the `Theorem.Register.ImplicationIntroduction` ABI call.
pub const ABI_THEOREM_REGISTER_IMPLICATION_INTRODUCTION_NAME: &str =
    "__theorem_register_implication_introduction";
/// The name of the `Theorem.Register.ImplicationElimination` ABI call.
pub const ABI_THEOREM_REGISTER_IMPLICATION_ELIMINATION_NAME: &str =
    "__theorem_register_implication_elimination";

/// The name of the `Theorem.Register.IffIntroduction` ABI call.
pub const ABI_THEOREM_REGISTER_IFF_INTRODUCTION_NAME: &str =
    "__theorem_register_iff_elimination";
/// The name of the `Theorem.Register.IffLeftElimination` ABI call.
pub const ABI_THEOREM_REGISTER_IFF_LEFT_ELIMINATION_NAME: &str =
    "__theorem_register_iff_left_elimination";

/// The name of the `Theorem.Register.NegationIntroduction` ABI call.
pub const ABI_THEOREM_REGISTER_NEGATION_INTRODUCTION_NAME: &str =
    "__theorem_register_negation_introduction";
/// The name of the `Theorem.Register.NegationElimination` ABI call.
pub const ABI_THEOREM_REGISTER_NEGATION_ELIMINATION_NAME: &str =
    "__theorem_register_negation_elimination";

/// The name of the `Theorem.Register.ForallIntroduction` ABI call.
pub const ABI_THEOREM_REGISTER_FORALL_INTRODUCTION_NAME: &str =
    "__theorem_register_forall_introduction";
/// The name of the `Theorem.Register.ForallElimination` ABI call.
pub const ABI_THEOREM_REGISTER_FORALL_ELIMINATION_NAME: &str =
    "__theorem_register_forall_elimination";
/// The name of the `Theorem.Register.ExistsIntroduction` ABI call.
pub const ABI_THEOREM_REGISTER_EXISTS_INTRODUCTION_NAME: &str =
    "__theorem_register_exists_introduction";
/// The name of the `Theorem.Register.ExistsElimination` ABI call.
pub const ABI_THEOREM_REGISTER_EXISTS_ELIMINATION_NAME: &str =
    "__theorem_register_exists_elimination";
/// The name of the `Theorem.Register.Weaken` ABI call.
pub const ABI_THEOREM_REGISTER_WEAKEN_NAME: &str = "__theorem_register_weaken";

/// The name of the `Theorem.Split.Hypotheses` ABI call.
pub const ABI_THEOREM_SPLIT_HYPOTHESES_NAME: &str =
    "__theorem_split_hypotheses";
/// The name of the `Theorem.Split.Conclusion` ABI call.
pub const ABI_THEOREM_SPLIT_CONCLUSION_NAME: &str =
    "__theorem_split_conclusion";

/// The index of the `Theorem.IsRegistered` ABI call.
pub const ABI_THEOREM_IS_REGISTERED_INDEX: usize = 57;

/// The index of the `Theorem.Register.Assumption` ABI call.
pub const ABI_THEOREM_REGISTER_ASSUMPTION_INDEX: usize = 58;

/// The index of the `Theorem.Register.Reflexivity` ABI call.
pub const ABI_THEOREM_REGISTER_REFLEXIVITY_INDEX: usize = 59;
/// The index of the `Theorem.Register.Symmetry` ABI call.
pub const ABI_THEOREM_REGISTER_SYMMETRY_INDEX: usize = 60;
/// The index of the `Theorem.Register.Transitivity` ABI call.
pub const ABI_THEOREM_REGISTER_TRANSITIVITY_INDEX: usize = 61;
/// The index of the `Theorem.Register.Beta` ABI call.
pub const ABI_THEOREM_REGISTER_BETA_INDEX: usize = 62;
/// The index of the `Theorem.Register.Eta` ABI call.
pub const ABI_THEOREM_REGISTER_ETA_INDEX: usize = 63;
/// The index of the `Theorem.Register.Application` ABI call.
pub const ABI_THEOREM_REGISTER_APPLICATION_INDEX: usize = 64;
/// The index of the `Theorem.Register.Lambda` ABI call.
pub const ABI_THEOREM_REGISTER_LAMBDA_INDEX: usize = 65;

/// The index of the `Theorem.Register.Substitution` ABI call.
pub const ABI_THEOREM_REGISTER_SUBSTITUTION_INDEX: usize = 66;
/// The index of the `Theorem.Register.TypeSubstitution` ABI call.
pub const ABI_THEOREM_REGISTER_TYPE_SUBSTITUTION_INDEX: usize = 67;

/// The index of the `Theorem.Register.TruthIntroduction` ABI call.
pub const ABI_THEOREM_REGISTER_TRUTH_INTRODUCTION_INDEX: usize = 68;
/// The index of the `Theorem.Register.FalsityElimination` ABI call.
pub const ABI_THEOREM_REGISTER_FALSITY_ELIMINATION_INDEX: usize = 69;

/// The index of the `Theorem.Register.ConjunctionIntroduction` ABI call.
pub const ABI_THEOREM_REGISTER_CONJUNCTION_INTRODUCTION_INDEX: usize = 70;
/// The index of the `Theorem.Register.ConjunctionLeftElimination` ABI call.
pub const ABI_THEOREM_REGISTER_CONJUNCTION_LEFT_ELIMINATION_INDEX: usize = 71;
/// The index of the `Theorem.Register.ConjunctionRightElimination` ABI call.
pub const ABI_THEOREM_REGISTER_CONJUNCTION_RIGHT_ELIMINATION_INDEX: usize = 72;

/// The index of the `Theorem.Register.DisjunctionElimination` ABI call.
pub const ABI_THEOREM_REGISTER_DISJUNCTION_ELIMINATION_INDEX: usize = 73;
/// The index of the `Theorem.Register.DisjunctionLeftIntroduction` ABI call.
pub const ABI_THEOREM_REGISTER_DISJUNCTION_LEFT_INTRODUCTION_INDEX: usize = 74;
/// The index of the `Theorem.Register.DisjunctionRightIntroduction` ABI call.
pub const ABI_THEOREM_REGISTER_DISJUNCTION_RIGHT_INTRODUCTION_INDEX: usize = 75;

/// The index of the `Theorem.Register.ImplicationIntroduction` ABI call.
pub const ABI_THEOREM_REGISTER_IMPLICATION_INTRODUCTION_INDEX: usize = 76;
/// The index of the `Theorem.Register.ImplicationElimination` ABI call.
pub const ABI_THEOREM_REGISTER_IMPLICATION_ELIMINATION_INDEX: usize = 77;

/// The index of the `Theorem.Register.IffIntroduction` ABI call.
pub const ABI_THEOREM_REGISTER_IFF_INTRODUCTION_INDEX: usize = 78;
/// The index of the `Theorem.Register.IffLeftElimination` ABI call.
pub const ABI_THEOREM_REGISTER_IFF_LEFT_ELIMINATION_INDEX: usize = 79;

/// The index of the `Theorem.Register.NegationIntroduction` ABI call.
pub const ABI_THEOREM_REGISTER_NEGATION_INTRODUCTION_INDEX: usize = 80;
/// The index of the `Theorem.Register.NegationElimination` ABI call.
pub const ABI_THEOREM_REGISTER_NEGATION_ELIMINATION_INDEX: usize = 81;

/// The index of the `Theorem.Register.ForallIntroduction` ABI call.
pub const ABI_THEOREM_REGISTER_FORALL_INTRODUCTION_INDEX: usize = 82;
/// The index of the `Theorem.Register.ForallElimination` ABI call.
pub const ABI_THEOREM_REGISTER_FORALL_ELIMINATION_INDEX: usize = 83;
/// The index of the `Theorem.Register.ExistsIntroduction` ABI call.
pub const ABI_THEOREM_REGISTER_EXISTS_INTRODUCTION_INDEX: usize = 84;
/// The index of the `Theorem.Register.ExistsElimination` ABI call.
pub const ABI_THEOREM_REGISTER_EXISTS_ELIMINATION_INDEX: usize = 85;
/// The index of the `Theorem.Register.Weaken` ABI call.
pub const ABI_THEOREM_REGISTER_WEAKEN_INDEX: usize = 86;

/// The index of the `Theorem.Split.Hypotheses` ABI call.
pub const ABI_THEOREM_SPLIT_HYPOTHESES_INDEX: usize = 87;
/// The index of the `Theorem.Split.Conclusion` ABI call.
pub const ABI_THEOREM_SPLIT_CONCLUSION_INDEX: usize = 88;

////////////////////////////////////////////////////////////////////////////////
// Errors and traps.
////////////////////////////////////////////////////////////////////////////////

/// Runtime traps are unrecoverable errors raised by the WASM program host.
/// These are equivalent, essentially, to kernel panics in a typical operating
/// system.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RuntimeTrap {
    /// The WASM guest's memory was not registered with the runtime state.
    MemoryNotRegistered,
    /// An attempted read from the WASM guest's heap failed.
    MemoryReadFailed,
    /// An attempted write to the WASM guest's heap failed.
    MemoryWriteFailed,
    /// The WASM guest program tried to call a function that does not exist.
    NoSuchFunction,
    /// A type-signature check on a host-function failed.
    SignatureFailure,
}

/// Pretty-printing for `RuntimeTrap` values.
impl Display for RuntimeTrap {
    fn fmt(&self, f: &mut Formatter) -> Result<(), DisplayError> {
        match self {
            RuntimeTrap::NoSuchFunction => write!(f, "NoSuchFunction"),
            RuntimeTrap::SignatureFailure => write!(f, "SignatureFailure"),
            RuntimeTrap::MemoryNotRegistered => {
                write!(f, "MemoryNotRegistered")
            }
            RuntimeTrap::MemoryReadFailed => write!(f, "MemoryReadFailed"),
            RuntimeTrap::MemoryWriteFailed => write!(f, "MemoryWriteFailed"),
        }
    }
}

impl HostError for KernelErrorCode {}
impl HostError for RuntimeTrap {}

/// Lifts a kernel error into an error that can be passed back to the WASM
/// program.
#[inline]
pub fn host_error(code: KernelErrorCode) -> WasmiError {
    WasmiError::Host(Box::new(code))
}

/// Creates a WASMI `Trap` type from a `RuntimeTrap`.
#[inline]
pub fn host_trap(trap: RuntimeTrap) -> Trap {
    Trap::new(TrapKind::Host(Box::new(trap)))
}

////////////////////////////////////////////////////////////////////////////////
// The WASMI runtime state.
////////////////////////////////////////////////////////////////////////////////

/// The WASMI runtime state, which is a thin wrapper around the kerne's own
/// runtime state, adding a reference to the guest WASM program's memory module,
/// to enable host functions to read-from and write-to the memory module
/// directly.
#[derive(Debug)]
pub struct WasmiRuntimeState {
    /// The kernel's runtime state.
    kernel: RefCell<KernelRuntimeState>,
    /// The memory instance of the executing WASM guest program.
    memory: Option<RefCell<MemoryInstance>>,
}

impl Default for WasmiRuntimeState {
    #[inline]
    fn default() -> Self {
        Self {
            kernel: RefCell::new(Default::default()),
            memory: None,
        }
    }
}

impl WasmiRuntimeState {
    /// Constructs a new instance of a `WasmiRuntimeState` with the kernel state
    /// intitialized to its correct initial state, and the reference to the WASM
    /// guest's memory set to `None`.
    #[inline]
    pub fn new() -> Self {
        Default::default()
    }

    ////////////////////////////////////////////////////////////////////////////
    // Memory-related functionality.
    ////////////////////////////////////////////////////////////////////////////

    /// Returns `true` iff the WASM guest's memory module has been registered.
    #[inline]
    pub fn is_memory_registered(&self) -> bool {
        self.memory.is_some()
    }

    /// Registers the WASM guest's memory module with the runtime state.
    #[inline]
    pub fn set_memory(&mut self, instance: MemoryInstance) -> &mut Self {
        self.memory = Some(RefCell::new(instance));
        self
    }

    /// Writes a buffer of byte values, `bytes`, to the WASM guest program's
    /// memory starting at the provided `address`.
    ///
    /// # Errors
    ///
    /// Returns `Err(RuntimeTrap::MemoryNotRegistered)` if the WASM guest's
    /// memory module has not been registered with the runtime state.
    ///
    /// Returns `Err(RuntimeTrap::MemoryWriteFailed)` if the write to memory at
    /// address, `address`, failed.
    fn write_bytes<T>(
        &self,
        address: T,
        bytes: &[u8],
    ) -> Result<(), RuntimeTrap>
    where
        T: Into<semantic_types::Pointer>,
    {
        let memory = match &self.memory {
            None => return Err(RuntimeTrap::MemoryNotRegistered),
            Some(memory) => memory,
        };

        memory
            .borrow_mut()
            .set(address.into(), bytes)
            .map_err(|_e| RuntimeTrap::MemoryWriteFailed)?;

        Ok(())
    }

    /// Writes a `u64` value to the WASM guest's memory module at a specified
    /// address.
    ///
    /// # Errors
    ///
    /// Returns `Err(RuntimeTrap::MemoryNotRegistered)` if the WASM guest's
    /// memory module has not been registered with the runtime state.
    ///
    /// Returns `Err(RuntimeTrap::MemoryWriteFailed)` if the write to memory at
    /// address, `address`, failed.
    fn write_u64<T, U>(&self, address: T, value: U) -> Result<(), RuntimeTrap>
    where
        T: Into<semantic_types::Pointer>,
        U: Into<u64>,
    {
        let mut buffer = Vec::new();
        LittleEndian::write_u64(&mut buffer, value.into());

        self.write_bytes(address, &buffer)
    }

    /// Writes a collection of `u64` values to the WASM guest's memory
    /// module starting at a specified address.
    ///
    /// # Errors
    ///
    /// Returns `Err(RuntimeTrap::MemoryNotRegistered)` if the WASM guest's
    /// memory module has not been registered with the runtime state.
    ///
    /// Returns `Err(RuntimeTrap::MemoryWriteFailed)` if the write to memory at
    /// address, `address`, failed.
    fn write_u64s<T, U>(
        &self,
        address: T,
        values: Vec<U>,
    ) -> Result<(), RuntimeTrap>
    where
        T: Into<semantic_types::Pointer>,
        U: Into<u64> + Clone,
    {
        let mut address = address.into();

        for v in values.iter().cloned() {
            self.write_u64(address, v)?;
            address += 8;
        }

        Ok(())
    }

    /// Writes a `bool` value to the WASM guest's memory module at a specified
    /// address.
    ///
    /// # Errors
    ///
    /// Returns `Err(RuntimeTrap::MemoryNotRegistered)` if the WASM guest's
    /// memory module has not been registered with the runtime state.
    ///
    /// Returns `Err(RuntimeTrap::MemoryWriteFailed)` if the write to memory at
    /// address, `address`, failed.
    fn write_bool<T, U>(&self, address: T, value: U) -> Result<(), RuntimeTrap>
    where
        T: Into<semantic_types::Pointer>,
        U: Into<bool>,
    {
        let mut buffer = Vec::new();
        LittleEndian::write_u32(&mut buffer, value.into() as u32);

        self.write_bytes(address, &buffer)
    }

    /// Writes a handle to the WASM guest's memory module at a specified
    /// address.
    ///
    /// # Errors
    ///
    /// Returns `Err(RuntimeTrap::MemoryNotRegistered)` if the WASM guest's
    /// memory module has not been registered with the runtime state.
    ///
    /// Returns `Err(RuntimeTrap::MemoryWriteFailed)` if the write to memory at
    /// address, `address`, failed.
    #[inline]
    fn write_handle<T, U, V>(
        &self,
        address: T,
        handle: U,
    ) -> Result<(), RuntimeTrap>
    where
        T: Into<semantic_types::Pointer>,
        U: Into<Handle<V>>,
        V: tags::IsTag,
    {
        self.write_u64(address, *handle.into() as u64)
    }

    /// Writes a collection of handles to the WASM guest's memory module
    /// starting at a specified address.
    ///
    /// # Errors
    ///
    /// Returns `Err(RuntimeTrap::MemoryNotRegistered)` if the WASM guest's
    /// memory module has not been registered with the runtime state.
    ///
    /// Returns `Err(RuntimeTrap::MemoryWriteFailed)` if the write to memory at
    /// address, `address`, failed.
    fn write_handles<T, U, V>(
        &self,
        address: T,
        handles: Vec<U>,
    ) -> Result<(), RuntimeTrap>
    where
        T: Into<semantic_types::Pointer>,
        U: Into<Handle<V>>,
        V: tags::IsTag,
    {
        let mut address = address.into();

        for handle in handles {
            self.write_handle(address, handle)?;
            address += 8;
        }

        Ok(())
    }

    /// Reads a fixed `byte_count` of bytes from the WASM guest's memory module
    /// at a specified `address`.
    ///
    /// # Errors
    ///
    /// Returns `Err(RuntimeTrap::MemoryNotRegistered)` if the WASM guest's
    /// memory module has not been registered with the runtime state.
    ///
    /// Returns `Err(RuntimeTrap::MemoryReadFailed)` if the read from memory at
    /// address, `address`, failed.
    fn read_bytes<T, U>(
        &self,
        address: T,
        byte_count: U,
    ) -> Result<Vec<u8>, RuntimeTrap>
    where
        T: Into<semantic_types::Pointer>,
        U: Into<usize>,
    {
        let memory = match &self.memory {
            None => return Err(RuntimeTrap::MemoryNotRegistered),
            Some(memory) => memory,
        };

        let bytes = memory
            .borrow()
            .get(address.into(), byte_count.into())
            .map_err(|_e| RuntimeTrap::MemoryReadFailed)?;

        Ok(bytes)
    }

    /// Reads a `u64` value from the WASM guest's memory module at a specified
    /// `address`.
    ///
    /// # Errors
    ///
    /// Returns `Err(RuntimeTrap::MemoryNotRegistered)` if the WASM guest's
    /// memory module has not been registered with the runtime state.
    ///
    /// Returns `Err(RuntimeTrap::MemoryReadFailed)` if the read from memory at
    /// address, `address`, failed.
    #[inline]
    fn read_u64<T>(&self, address: T) -> Result<u64, RuntimeTrap>
    where
        T: Into<semantic_types::Pointer>,
    {
        let buffer = self.read_bytes(address, size_of::<u64>())?;
        Ok(LittleEndian::read_u64(&buffer))
    }

    /// Reads multiple `u64` values, as described by `count`, from the WASM
    /// guest's memory module at a specified `address`.
    ///
    /// # Errors
    ///
    /// Returns `Err(RuntimeTrap::MemoryNotRegistered)` if the WASM guest's
    /// memory module has not been registered with the runtime state.
    ///
    /// Returns `Err(RuntimeTrap::MemoryReadFailed)` if the read from memory at
    /// address, `address`, failed.
    fn read_u64s<T, U>(
        &self,
        address: T,
        count: U,
    ) -> Result<Vec<u64>, RuntimeTrap>
    where
        T: Into<u32>,
        U: Into<usize>,
    {
        let mut accum = Vec::new();
        let mut address = address.into();

        for _c in 0..count.into() {
            let handle = self.read_u64(address)?;
            accum.push(handle);
            address += 8;
        }

        Ok(accum)
    }

    /// Reads a `Handle` from the WASM guest's memory module at a specified
    /// `address`.
    ///
    /// # Errors
    ///
    /// Returns `Err(RuntimeTrap::MemoryNotRegistered)` if the WASM guest's
    /// memory module has not been registered with the runtime state.
    ///
    /// Returns `Err(RuntimeTrap::MemoryReadFailed)` if the read from memory at
    /// address, `address`, failed.
    #[inline]
    fn read_handle<T, U>(&self, address: U) -> Result<Handle<T>, RuntimeTrap>
    where
        T: tags::IsTag,
        U: Into<u32>,
    {
        Ok(Handle::from(self.read_u64(address)? as usize))
    }

    /// Reads multiple `Handle` values, as described by `count`, from the WASM
    /// guest's memory module at a specified `address`.
    ///
    /// # Errors
    ///
    /// Returns `Err(RuntimeTrap::MemoryNotRegistered)` if the WASM guest's
    /// memory module has not been registered with the runtime state.
    ///
    /// Returns `Err(RuntimeTrap::MemoryReadFailed)` if the read from memory at
    /// address, `address`, failed.
    fn read_handles<T, U, V>(
        &self,
        address: U,
        count: V,
    ) -> Result<Vec<Handle<T>>, RuntimeTrap>
    where
        T: tags::IsTag,
        U: Into<u32>,
        V: Into<usize>,
    {
        let mut accum = Vec::new();
        let mut address = address.into();

        for _c in 0..count.into() {
            let handle = self.read_handle(address)?;
            accum.push(handle);
            address += 8;
        }

        Ok(accum)
    }

    ////////////////////////////////////////////////////////////////////////////
    // Kernel-related functionality.
    ////////////////////////////////////////////////////////////////////////////

    /// Lifting of the `type_former_resolve` function.
    #[inline]
    fn type_former_resolve<T>(&self, handle: T) -> Option<usize>
    where
        T: Borrow<Handle<tags::TypeFormer>>,
    {
        self.kernel.borrow().type_former_resolve(handle).cloned()
    }

    /// Lifting of the `type_former_is_registered` function.
    #[inline]
    fn type_former_is_registered<T>(&self, handle: T) -> bool
    where
        T: Borrow<Handle<tags::TypeFormer>>,
    {
        self.kernel.borrow().type_former_is_registered(handle)
    }

    /// Lifting of the `type_former_register` function.
    #[inline]
    fn type_former_register<T>(&self, arity: T) -> Handle<tags::TypeFormer>
    where
        T: Into<usize> + Clone,
    {
        self.kernel.borrow_mut().type_former_register(arity)
    }

    /// Lifting of the `type_register_variable` function.
    #[inline]
    fn type_register_variable<T>(&self, name: T) -> Handle<tags::Type>
    where
        T: Into<Name> + Clone,
    {
        self.kernel.borrow_mut().type_register_variable(name)
    }

    /// Lifting of the `type_register_combination` function.
    #[inline]
    fn type_register_combination<T, U>(
        &self,
        type_former: T,
        arguments: Vec<U>,
    ) -> Result<Handle<tags::Type>, KernelErrorCode>
    where
        T: Into<Handle<tags::TypeFormer>> + Clone,
        U: Into<Handle<tags::Type>> + Clone,
    {
        self.kernel.borrow_mut().type_register_combination(
            type_former.into(),
            arguments.iter().cloned().map(|a| a.into()).collect(),
        )
    }

    /// Lifting of the `type_register_function` function.
    #[inline]
    fn type_register_function<T>(
        &self,
        domain: T,
        range: T,
    ) -> Result<Handle<tags::Type>, KernelErrorCode>
    where
        T: Into<Handle<tags::Type>>,
    {
        self.kernel
            .borrow_mut()
            .type_register_function(domain.into(), range.into())
    }

    /// Lifting of the `type_is_registered` function.
    #[inline]
    fn type_is_registered<T>(&self, handle: T) -> bool
    where
        T: Borrow<Handle<tags::Type>>,
    {
        self.kernel.borrow().type_is_registered(handle)
    }

    /// Lifting of the `type_split_variable` function.
    #[inline]
    fn type_split_variable<T>(&self, handle: T) -> Result<Name, KernelErrorCode>
    where
        T: Borrow<Handle<tags::Type>>,
    {
        self.kernel
            .borrow()
            .type_split_variable(handle)
            .map(|n| n.clone())
    }

    /// Lifting of the `type_split_combination` function.
    #[inline]
    fn type_split_combination<T>(
        &self,
        handle: T,
    ) -> Result<
        (Handle<tags::TypeFormer>, Vec<Handle<tags::Type>>),
        KernelErrorCode,
    >
    where
        T: Borrow<Handle<tags::Type>>,
    {
        self.kernel
            .borrow()
            .type_split_combination(handle)
            .map(|(f, a)| (f.clone(), a.clone()))
    }

    /// Lifting of the `type_split_function` function.
    #[inline]
    fn type_split_function<T>(
        &self,
        handle: T,
    ) -> Result<(Handle<tags::Type>, Handle<tags::Type>), KernelErrorCode>
    where
        T: Borrow<Handle<tags::Type>>,
    {
        self.kernel
            .borrow()
            .type_split_function(handle)
            .map(|(d, r)| (d.clone(), r.clone()))
    }

    /// Lifting of the `type_test_variable` function.
    #[inline]
    fn type_test_variable<T>(&self, handle: T) -> Result<bool, KernelErrorCode>
    where
        T: Borrow<Handle<tags::Type>>,
    {
        self.kernel.borrow().type_test_variable(handle)
    }

    /// Lifting of the `type_test_combination` function.
    #[inline]
    fn type_test_combination<T>(
        &self,
        handle: T,
    ) -> Result<bool, KernelErrorCode>
    where
        T: Borrow<Handle<tags::Type>>,
    {
        self.kernel.borrow().type_test_combination(handle)
    }

    /// Lifting of the `type_test_function` function.
    #[inline]
    fn type_test_function<T>(&self, handle: T) -> Result<bool, KernelErrorCode>
    where
        T: Borrow<Handle<tags::Type>>,
    {
        self.kernel.borrow().type_test_function(handle)
    }

    /// Lifting of the `type_variables` function.
    #[inline]
    fn type_variables<T>(&self, handle: T) -> Result<Vec<Name>, KernelErrorCode>
    where
        T: Borrow<Handle<tags::Type>>,
    {
        self.kernel
            .borrow_mut()
            .type_variables(handle)
            .map(|v| v.iter().map(|e| *e.clone()).collect())
    }

    /// Lifting of the `type_substitute` function.
    #[inline]
    fn type_substitute<T, U, V>(
        &self,
        handle: T,
        sigma: Vec<(U, V)>,
    ) -> Result<Handle<tags::Type>, KernelErrorCode>
    where
        T: Borrow<Handle<tags::Type>> + Clone,
        U: Into<Name> + Clone + Debug,
        V: Into<Handle<tags::Type>> + Clone + Debug,
    {
        self.kernel.borrow_mut().type_substitute(handle, sigma)
    }

    /// Lifting of the `constant_register` function.
    #[inline]
    fn constant_register<T>(
        &self,
        handle: T,
    ) -> Result<Handle<tags::Constant>, KernelErrorCode>
    where
        T: Into<Handle<tags::Type>> + Clone,
    {
        self.kernel.borrow_mut().constant_register(handle)
    }

    /// Lifting of the `constant_resolve` function.
    #[inline]
    fn constant_resolve<T>(
        &self,
        handle: T,
    ) -> Result<Handle<tags::Type>, KernelErrorCode>
    where
        T: Borrow<Handle<tags::Constant>>,
    {
        self.kernel
            .borrow()
            .constant_resolve(handle)
            .map(|e| e.clone())
    }

    /// Lifting of the `constant_is_registered` function.
    #[inline]
    fn constant_is_registered<T>(&self, handle: T) -> bool
    where
        T: Borrow<Handle<tags::Constant>>,
    {
        self.kernel.borrow().constant_is_registered(handle)
    }

    /// Lifting of the `term_register_variable` function.
    #[inline]
    fn term_register_variable<T, U>(
        &self,
        name: T,
        tau: U,
    ) -> Result<Handle<tags::Term>, KernelErrorCode>
    where
        T: Into<Name> + Clone,
        U: Into<Handle<tags::Type>> + Clone,
    {
        self.kernel.borrow_mut().term_register_variable(name, tau)
    }

    /// Lifting of the `term_register_constant` function.
    #[inline]
    fn term_register_constant<T, U, V>(
        &self,
        constant: T,
        substitution: Vec<(U, V)>,
    ) -> Result<Handle<tags::Term>, KernelErrorCode>
    where
        T: Into<Handle<tags::Constant>> + Clone,
        U: Into<Name> + Clone + Debug,
        V: Into<Handle<tags::Type>> + Clone + Debug,
    {
        self.kernel
            .borrow_mut()
            .term_register_constant(constant, substitution)
    }

    /// Lifting of the `term_register_application` function.
    #[inline]
    fn term_register_application<T, U>(
        &self,
        left: T,
        right: U,
    ) -> Result<Handle<tags::Term>, KernelErrorCode>
    where
        T: Into<Handle<tags::Term>> + Clone,
        U: Into<Handle<tags::Term>> + Clone,
    {
        self.kernel
            .borrow_mut()
            .term_register_application(left, right)
    }

    /// Lifting of the `term_register_lambda` function.
    #[inline]
    fn term_register_lambda<T, U, V>(
        &self,
        name: T,
        tau: U,
        body: V,
    ) -> Result<Handle<tags::Term>, KernelErrorCode>
    where
        T: Into<Name> + Clone,
        U: Into<Handle<tags::Type>> + Clone,
        V: Into<Handle<tags::Term>> + Clone,
    {
        self.kernel
            .borrow_mut()
            .term_register_lambda(name, tau, body)
    }

    /// Lifting of the `term_register_negation` function.
    #[inline]
    fn term_register_negation<T>(
        &self,
        body: T,
    ) -> Result<Handle<tags::Term>, KernelErrorCode>
    where
        T: Into<Handle<tags::Term>> + Clone,
    {
        self.kernel.borrow_mut().term_register_negation(body)
    }

    /// Lifting of the `term_register_conjunction` function.
    #[inline]
    fn term_register_conjunction<T, U>(
        &self,
        left: T,
        right: U,
    ) -> Result<Handle<tags::Term>, KernelErrorCode>
    where
        T: Into<Handle<tags::Term>> + Clone,
        U: Into<Handle<tags::Term>> + Clone,
    {
        self.kernel
            .borrow_mut()
            .term_register_conjunction(left, right)
    }

    /// Lifting of the `term_register_disjunction` function.
    #[inline]
    fn term_register_disjunction<T, U>(
        &self,
        left: T,
        right: U,
    ) -> Result<Handle<tags::Term>, KernelErrorCode>
    where
        T: Into<Handle<tags::Term>> + Clone,
        U: Into<Handle<tags::Term>> + Clone,
    {
        self.kernel
            .borrow_mut()
            .term_register_disjunction(left, right)
    }

    /// Lifting of the `term_register_implication` function.
    #[inline]
    fn term_register_implication<T, U>(
        &self,
        left: T,
        right: U,
    ) -> Result<Handle<tags::Term>, KernelErrorCode>
    where
        T: Into<Handle<tags::Term>> + Clone,
        U: Into<Handle<tags::Term>> + Clone,
    {
        self.kernel
            .borrow_mut()
            .term_register_implication(left, right)
    }

    /// Lifting of the `term_register_equality` function.
    #[inline]
    fn term_register_equality<T, U>(
        &self,
        left: T,
        right: U,
    ) -> Result<Handle<tags::Term>, KernelErrorCode>
    where
        T: Into<Handle<tags::Term>> + Clone,
        U: Into<Handle<tags::Term>> + Clone,
    {
        self.kernel.borrow_mut().term_register_equality(left, right)
    }

    /// Lifting of the `term_register_forall` function.
    #[inline]
    fn term_register_forall<T, U, V>(
        &self,
        name: T,
        tau: U,
        body: V,
    ) -> Result<Handle<tags::Term>, KernelErrorCode>
    where
        T: Into<Name> + Clone,
        U: Into<Handle<tags::Type>> + Clone,
        V: Into<Handle<tags::Term>> + Clone,
    {
        self.kernel
            .borrow_mut()
            .term_register_forall(name, tau, body)
    }

    /// Lifting of the `term_register_exists` function.
    #[inline]
    fn term_register_exists<T, U, V>(
        &self,
        name: T,
        tau: U,
        body: V,
    ) -> Result<Handle<tags::Term>, KernelErrorCode>
    where
        T: Into<Name> + Clone,
        U: Into<Handle<tags::Type>> + Clone,
        V: Into<Handle<tags::Term>> + Clone,
    {
        self.kernel
            .borrow_mut()
            .term_register_exists(name, tau, body)
    }

    /// Lifting of the `term_split_variable` function.
    #[inline]
    fn term_split_variable<T>(
        &self,
        handle: T,
    ) -> Result<(Name, Handle<tags::Type>), KernelErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        self.kernel
            .borrow()
            .term_split_variable(handle)
            .map(|(n, t)| (n.clone(), t.clone()))
    }

    /// Lifting of the `term_split_constant` function.
    #[inline]
    fn term_split_constant<T>(
        &self,
        handle: T,
    ) -> Result<(Handle<tags::Constant>, Handle<tags::Type>), KernelErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        self.kernel
            .borrow()
            .term_split_constant(handle)
            .map(|(c, t)| (c.clone(), t.clone()))
    }

    /// Lifting of the `term_split_application` function.
    #[inline]
    fn term_split_application<T>(
        &self,
        handle: T,
    ) -> Result<(Handle<tags::Term>, Handle<tags::Term>), KernelErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        self.kernel
            .borrow()
            .term_split_application(handle)
            .map(|(l, r)| (l.clone(), r.clone()))
    }

    /// Lifting of the `term_split_lambda` function.
    #[inline]
    fn term_split_lambda<T>(
        &self,
        handle: T,
    ) -> Result<(Name, Handle<tags::Type>, Handle<tags::Term>), KernelErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        self.kernel
            .borrow()
            .term_split_lambda(handle)
            .map(|(n, t, b)| (n.clone(), t.clone(), b.clone()))
    }

    /// Lifting of the `term_split_negation` function.
    #[inline]
    fn term_split_negation<T>(
        &self,
        handle: T,
    ) -> Result<Handle<tags::Term>, KernelErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        self.kernel
            .borrow()
            .term_split_negation(handle)
            .map(|n| n.clone())
    }

    /// Lifting of the `term_split_conjunction` function.
    #[inline]
    fn term_split_conjunction<T>(
        &self,
        handle: T,
    ) -> Result<(Handle<tags::Term>, Handle<tags::Term>), KernelErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        self.kernel
            .borrow()
            .term_split_conjunction(handle)
            .map(|(l, r)| (l.clone(), r.clone()))
    }

    /// Lifting of the `term_split_disjunction` function.
    #[inline]
    fn term_split_disjunction<T>(
        &self,
        handle: T,
    ) -> Result<(Handle<tags::Term>, Handle<tags::Term>), KernelErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        self.kernel
            .borrow()
            .term_split_disjunction(handle)
            .map(|(l, r)| (l.clone(), r.clone()))
    }

    /// Lifting of the `term_split_implication` function.
    #[inline]
    fn term_split_implication<T>(
        &self,
        handle: T,
    ) -> Result<(Handle<tags::Term>, Handle<tags::Term>), KernelErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        self.kernel
            .borrow()
            .term_split_implication(handle)
            .map(|(l, r)| (l.clone(), r.clone()))
    }

    /// Lifting of the `term_split_equality` function.
    #[inline]
    fn term_split_equality<T>(
        &self,
        handle: T,
    ) -> Result<(Handle<tags::Term>, Handle<tags::Term>), KernelErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        self.kernel
            .borrow()
            .term_split_equality(handle)
            .map(|(l, r)| (l.clone(), r.clone()))
    }

    /// Lifting of the `term_split_forall` function.
    #[inline]
    fn term_split_forall<T>(
        &self,
        handle: T,
    ) -> Result<(Name, Handle<tags::Type>, Handle<tags::Term>), KernelErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        self.kernel
            .borrow()
            .term_split_forall(handle)
            .map(|(n, t, b)| (n.clone(), t.clone(), b.clone()))
    }

    /// Lifting of the `term_split_exists` function.
    #[inline]
    fn term_split_exists<T>(
        &self,
        handle: T,
    ) -> Result<(Name, Handle<tags::Type>, Handle<tags::Term>), KernelErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        self.kernel
            .borrow()
            .term_split_exists(handle)
            .map(|(n, t, b)| (n.clone(), t.clone(), b.clone()))
    }

    /// Lifting of the `term_test_variable` function.
    #[inline]
    fn term_test_variable<T>(&self, handle: T) -> Result<bool, KernelErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        self.kernel.borrow().term_test_variable(handle)
    }

    /// Lifting of the `term_test_constant` function.
    #[inline]
    fn term_test_constant<T>(&self, handle: T) -> Result<bool, KernelErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        self.kernel.borrow().term_test_constant(handle)
    }

    /// Lifting of the `term_test_application` function.
    #[inline]
    fn term_test_application<T>(
        &self,
        handle: T,
    ) -> Result<bool, KernelErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        self.kernel.borrow().term_test_application(handle)
    }

    /// Lifting of the `term_test_lambda` function.
    #[inline]
    fn term_test_lambda<T>(&self, handle: T) -> Result<bool, KernelErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        self.kernel.borrow().term_test_lambda(handle)
    }

    /// Lifting of the `term_test_negation` function.
    #[inline]
    fn term_test_negation<T>(&self, handle: T) -> Result<bool, KernelErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        self.kernel.borrow().term_test_negation(handle)
    }

    /// Lifting of the `term_test_conjunction` function.
    #[inline]
    fn term_test_conjunction<T>(
        &self,
        handle: T,
    ) -> Result<bool, KernelErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        self.kernel.borrow().term_test_conjunction(handle)
    }

    /// Lifting of the `term_test_disjunction` function.
    #[inline]
    fn term_test_disjunction<T>(
        &self,
        handle: T,
    ) -> Result<bool, KernelErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        self.kernel.borrow().term_test_disjunction(handle)
    }

    /// Lifting of the `term_test_implication` function.
    #[inline]
    fn term_test_implication<T>(
        &self,
        handle: T,
    ) -> Result<bool, KernelErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        self.kernel.borrow().term_test_implication(handle)
    }

    /// Lifting of the `term_test_equality` function.
    #[inline]
    fn term_test_equality<T>(&self, handle: T) -> Result<bool, KernelErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        self.kernel.borrow().term_test_equality(handle)
    }

    /// Lifting of the `term_test_forall` function.
    #[inline]
    fn term_test_forall<T>(&self, handle: T) -> Result<bool, KernelErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        self.kernel.borrow().term_test_forall(handle)
    }

    /// Lifting of the `term_test_exists` function.
    #[inline]
    fn term_test_exists<T>(&self, handle: T) -> Result<bool, KernelErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        self.kernel.borrow().term_test_exists(handle)
    }

    /// Lifting of the `term_free_variables` function.
    #[inline]
    fn term_free_variables<T>(
        &self,
        handle: T,
    ) -> Result<Vec<(Name, Handle<tags::Type>)>, KernelErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        self.kernel.borrow().term_free_variables(handle).map(|v| {
            v.iter()
                .cloned()
                .map(|(n, t)| (n.clone(), t.clone()))
                .collect()
        })
    }

    /// Lifting of the `term_type_variables` function.
    #[inline]
    fn term_type_variables<T>(
        &self,
        handle: T,
    ) -> Result<Vec<Name>, KernelErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        self.kernel
            .borrow()
            .term_type_variables(handle)
            .map(|v| v.iter().cloned().cloned().collect())
    }

    /// Lifting of the `term_substitution` function.
    #[inline]
    fn term_substitution<T, U, V>(
        &self,
        handle: T,
        substitution: Vec<(U, V)>,
    ) -> Result<Handle<tags::Term>, KernelErrorCode>
    where
        T: Into<Handle<tags::Term>>,
        U: Into<Name> + Clone,
        V: Into<Handle<tags::Term>> + Clone,
    {
        self.kernel.borrow_mut().substitution(handle, substitution)
    }

    /// Lifting of the `term_type_substitution` function.
    #[inline]
    fn term_type_substitution<T, U, V>(
        &self,
        handle: T,
        substitution: Vec<(U, V)>,
    ) -> Result<Handle<tags::Term>, KernelErrorCode>
    where
        T: Into<Handle<tags::Term>>,
        U: Into<Name> + Clone,
        V: Into<Handle<tags::Type>> + Clone,
    {
        self.kernel
            .borrow_mut()
            .term_type_substitution(handle, substitution)
    }

    /// Lifting of the `term_type_infer` function.
    #[inline]
    fn term_type_infer<T>(
        &self,
        handle: T,
    ) -> Result<Handle<tags::Type>, KernelErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        self.kernel.borrow_mut().term_type_infer(handle)
    }

    /// Lifting of the `term_type_is_proposition` function.
    #[inline]
    fn term_type_is_proposition<T>(
        &self,
        handle: T,
    ) -> Result<bool, KernelErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        self.kernel.borrow_mut().term_type_is_proposition(handle)
    }

    /// Lifting of the `theorem_is_registered` function.
    #[inline]
    fn theorem_is_registered<T>(&self, handle: T) -> bool
    where
        T: Borrow<Handle<tags::Theorem>>,
    {
        self.kernel.borrow().theorem_is_registered(handle)
    }

    /// Lifting of the `theorem_register_assumption` function.
    #[inline]
    fn theorem_register_assumption<T, U>(
        &self,
        hyps_handles: Vec<T>,
        term_handle: U,
    ) -> Result<Handle<tags::Theorem>, KernelErrorCode>
    where
        T: Into<Handle<tags::Term>> + Clone,
        U: Into<Handle<tags::Term>> + Clone,
    {
        self.kernel
            .borrow_mut()
            .theorem_register_assumption(hyps_handles, term_handle)
    }

    /// Lifting of the `theorem_register_reflexivity` function.
    #[inline]
    fn theorem_register_reflexivity<T, U>(
        &self,
        hyps_handles: Vec<T>,
        term_handle: U,
    ) -> Result<Handle<tags::Theorem>, KernelErrorCode>
    where
        T: Into<Handle<tags::Term>> + Clone,
        U: Into<Handle<tags::Term>> + Clone,
    {
        self.kernel
            .borrow_mut()
            .theorem_register_reflexivity(hyps_handles, term_handle)
    }

    /// Lifting of the `theorem_register_symmetry` function.
    #[inline]
    fn theorem_register_symmetry<T>(
        &self,
        theorem_handle: T,
    ) -> Result<Handle<tags::Theorem>, KernelErrorCode>
    where
        T: Borrow<Handle<tags::Theorem>>,
    {
        self.kernel
            .borrow_mut()
            .theorem_register_symmetry(theorem_handle)
    }

    /// Lifting of the `theorem_register_transitivity` function.
    #[inline]
    fn theorem_register_transitivity<T, U>(
        &self,
        left_handle: T,
        right_handle: U,
    ) -> Result<Handle<tags::Theorem>, KernelErrorCode>
    where
        T: Borrow<Handle<tags::Theorem>>,
        U: Borrow<Handle<tags::Theorem>>,
    {
        self.kernel
            .borrow_mut()
            .theorem_register_transitivity(left_handle, right_handle)
    }

    /// Lifting of the `theorem_register_beta` function.
    #[inline]
    fn theorem_register_beta<T, U>(
        &self,
        hyps_handles: Vec<T>,
        term_handle: U,
    ) -> Result<Handle<tags::Theorem>, KernelErrorCode>
    where
        T: Into<Handle<tags::Term>> + Clone,
        U: Into<Handle<tags::Term>> + Clone,
    {
        self.kernel
            .borrow_mut()
            .theorem_register_beta(hyps_handles, term_handle)
    }

    /// Lifting of the `theorem_register_eta` function.
    #[inline]
    fn theorem_register_eta<T, U>(
        &self,
        hyps_handles: Vec<T>,
        term_handle: U,
    ) -> Result<Handle<tags::Theorem>, KernelErrorCode>
    where
        T: Into<Handle<tags::Term>> + Clone,
        U: Into<Handle<tags::Term>> + Clone,
    {
        self.kernel
            .borrow_mut()
            .theorem_register_eta(hyps_handles, term_handle)
    }

    /// Lifting of the `theorem_register_substitution` function.
    #[inline]
    fn theorem_register_substitution<T, U>(
        &self,
        theorem_handle: T,
        subst: Vec<(Name, U)>,
    ) -> Result<Handle<tags::Theorem>, KernelErrorCode>
    where
        T: Borrow<Handle<tags::Theorem>>,
        U: Into<Handle<tags::Term>> + Clone,
    {
        self.kernel
            .borrow_mut()
            .theorem_register_substitution(theorem_handle, subst)
    }

    /// Lifting of the `theorem_register_type_substitution` function.
    #[inline]
    fn theorem_register_type_substitution<T, U>(
        &self,
        theorem_handle: T,
        subst: Vec<(Name, U)>,
    ) -> Result<Handle<tags::Theorem>, KernelErrorCode>
    where
        T: Borrow<Handle<tags::Theorem>>,
        U: Into<Handle<tags::Type>> + Clone,
    {
        self.kernel
            .borrow_mut()
            .theorem_register_type_substitution(theorem_handle, subst)
    }

    /// Lifting of the `theorem_register_application` function.
    #[inline]
    fn theorem_register_application<T, U>(
        &self,
        left_handle: T,
        right_handle: U,
    ) -> Result<Handle<tags::Theorem>, KernelErrorCode>
    where
        T: Borrow<Handle<tags::Theorem>>,
        U: Borrow<Handle<tags::Theorem>>,
    {
        self.kernel
            .borrow_mut()
            .theorem_register_application(left_handle, right_handle)
    }

    /// Lifting of the `theorem_register_lambda` function.
    #[inline]
    fn theorem_register_lambda<T, U, V>(
        &self,
        name: T,
        type_handle: U,
        body_handle: V,
    ) -> Result<Handle<tags::Theorem>, KernelErrorCode>
    where
        T: Into<Name> + Clone,
        U: Into<Handle<tags::Type>> + Clone,
        V: Borrow<Handle<tags::Theorem>>,
    {
        self.kernel.borrow_mut().theorem_register_lambda(
            name,
            type_handle,
            body_handle,
        )
    }

    /// Lifting of the `theorem_register_truth_introduction` function.
    #[inline]
    fn theorem_register_truth_introduction<T>(
        &self,
        hyps_handles: Vec<T>,
    ) -> Result<Handle<tags::Theorem>, KernelErrorCode>
    where
        T: Into<Handle<tags::Term>> + Clone,
    {
        self.kernel
            .borrow_mut()
            .theorem_register_truth_introduction(hyps_handles)
    }

    /// Lifting of the `theorem_register_falsity_elimination` function.
    #[inline]
    fn theorem_register_falsity_elimination<T, U>(
        &self,
        theorem_handle: T,
        conclusion_handle: U,
    ) -> Result<Handle<tags::Theorem>, KernelErrorCode>
    where
        T: Borrow<Handle<tags::Theorem>>,
        U: Into<Handle<tags::Term>> + Clone,
    {
        self.kernel
            .borrow_mut()
            .theorem_register_falsity_elimination(
                theorem_handle,
                conclusion_handle,
            )
    }

    /// Lifting of the `theorem_register_conjunction_introduction` function.
    #[inline]
    fn theorem_register_conjunction_introduction<T, U>(
        &self,
        left_handle: T,
        right_handle: U,
    ) -> Result<Handle<tags::Theorem>, KernelErrorCode>
    where
        T: Borrow<Handle<tags::Theorem>>,
        U: Borrow<Handle<tags::Theorem>>,
    {
        self.kernel
            .borrow_mut()
            .theorem_register_conjunction_introduction(
                left_handle,
                right_handle,
            )
    }

    /// Lifting of the `theorem_register_conjunction_left_elimination` function.
    #[inline]
    fn theorem_register_conjunction_left_elimination<T>(
        &self,
        theorem_handle: T,
    ) -> Result<Handle<tags::Theorem>, KernelErrorCode>
    where
        T: Borrow<Handle<tags::Theorem>>,
    {
        self.kernel
            .borrow_mut()
            .theorem_register_conjunction_left_elimination(theorem_handle)
    }

    /// Lifting of the `theorem_register_conjunction_right_elimination` function.
    #[inline]
    fn theorem_register_conjunction_right_elimination<T>(
        &self,
        theorem_handle: T,
    ) -> Result<Handle<tags::Theorem>, KernelErrorCode>
    where
        T: Borrow<Handle<tags::Theorem>>,
    {
        self.kernel
            .borrow_mut()
            .theorem_register_conjunction_right_elimination(theorem_handle)
    }

    /// Lifting of the `theorem_register_disjunction_left_introduction`
    /// function.
    #[inline]
    fn theorem_register_disjunction_left_introduction<T, U>(
        &self,
        theorem_handle: T,
        term_handle: U,
    ) -> Result<Handle<tags::Theorem>, KernelErrorCode>
    where
        T: Borrow<Handle<tags::Theorem>>,
        U: Into<Handle<tags::Term>> + Clone,
    {
        self.kernel
            .borrow_mut()
            .theorem_register_disjunction_left_introduction(
                theorem_handle,
                term_handle,
            )
    }

    /// Lifting of the `theorem_register_disjunction_right_introduction`
    /// function.
    #[inline]
    fn theorem_register_disjunction_right_introduction<T, U>(
        &self,
        theorem_handle: T,
        term_handle: U,
    ) -> Result<Handle<tags::Theorem>, KernelErrorCode>
    where
        T: Borrow<Handle<tags::Theorem>>,
        U: Into<Handle<tags::Term>> + Clone,
    {
        self.kernel
            .borrow_mut()
            .theorem_register_disjunction_right_introduction(
                theorem_handle,
                term_handle,
            )
    }

    /// Lifting of the `theorem_split_conclusion` function.
    #[inline]
    fn theorem_split_conclusion<T>(
        &self,
        handle: T,
    ) -> Result<Handle<tags::Term>, KernelErrorCode>
    where
        T: Borrow<Handle<tags::Theorem>>,
    {
        self.kernel.borrow().theorem_split_conclusion(handle)
    }

    /// Lifting of the `theorem_split_hypotheses` function.
    #[inline]
    fn theorem_split_hypotheses<T>(
        &self,
        handle: T,
    ) -> Result<Vec<Handle<tags::Term>>, KernelErrorCode>
    where
        T: Borrow<Handle<tags::Theorem>>,
    {
        self.kernel.borrow().theorem_split_hypotheses(handle)
    }
}

////////////////////////////////////////////////////////////////////////////////
// Signature checking.
////////////////////////////////////////////////////////////////////////////////

/// Returns `true` iff the semantic function signature type described by
/// `params` and `ret` is implemented by the WASM type described by
/// `signature`.
fn check_signature(
    signature: &Signature,
    params: &[AbiType],
    ret: &Option<AbiType>,
) -> bool {
    let params = signature
        .params()
        .iter()
        .zip(params)
        .all(|(a, w)| w.implemented_by(a));

    match (ret, signature.return_type()) {
        (None, None) => params,
        (Some(a), Some(w)) => a.implemented_by(&w) && params,
        _otherwise => false,
    }
}

/// Checks the signature of the `TypeFormer.Resolve` ABI function.
#[inline]
fn check_type_former_resolve_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[AbiType::Handle, AbiType::Pointer],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `TypeFormer.Register` ABI function.
#[inline]
fn check_type_former_register_signature(signature: &Signature) -> bool {
    check_signature(signature, &[AbiType::Arity], &Some(AbiType::Handle))
}

/// Checks the signature of the `TypeFormer.IsRegistered` ABI function.
#[inline]
fn check_type_former_is_registered_signature(signature: &Signature) -> bool {
    check_signature(signature, &[AbiType::Handle], &Some(AbiType::Boolean))
}

/// Checks the signature of the `Type.Register.Variable` ABI function.
#[inline]
fn check_type_register_variable_signature(signature: &Signature) -> bool {
    check_signature(signature, &[AbiType::Name], &Some(AbiType::Handle))
}

/// Checks the signature of the `Type.Register.Combination` ABI function.
#[inline]
fn check_type_register_combination_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[
            AbiType::Handle,
            AbiType::Pointer,
            AbiType::Size,
            AbiType::Pointer,
        ],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Type.Register.Function` ABI function.
#[inline]
fn check_type_register_function_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[AbiType::Handle, AbiType::Handle, AbiType::Pointer],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Type.IsRegistered` ABI function.
#[inline]
fn check_type_is_registered_signature(signature: &Signature) -> bool {
    check_signature(signature, &[AbiType::Handle], &Some(AbiType::Boolean))
}

/// Checks the signature of the `Type.Split.Variable` ABI function.
#[inline]
fn check_type_split_variable_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[AbiType::Handle, AbiType::Pointer],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Type.Split.Combination` ABI function.
#[inline]
fn check_type_split_combination_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[
            AbiType::Handle,
            AbiType::Pointer,
            AbiType::Pointer,
            AbiType::Size,
        ],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Type.Split.Function` ABI function.
#[inline]
fn check_type_split_function_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[AbiType::Handle, AbiType::Pointer, AbiType::Pointer],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Type.Test.Variable` ABI function.
#[inline]
fn check_type_test_variable_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[AbiType::Handle, AbiType::Pointer],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Type.Test.Combination` ABI function.
#[inline]
fn check_type_test_combination_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[AbiType::Handle, AbiType::Pointer],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Type.Test.Function` ABI function.
#[inline]
fn check_type_test_function_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[AbiType::Handle, AbiType::Pointer],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Type.FTV` ABI function.
#[inline]
fn check_type_ftv_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[AbiType::Handle, AbiType::Pointer, AbiType::Size],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Type.Substitute` ABI function.
#[inline]
fn check_type_substitute_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[
            AbiType::Handle,
            AbiType::Pointer,
            AbiType::Size,
            AbiType::Pointer,
            AbiType::Size,
            AbiType::Pointer,
        ],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Constant.Register` ABI function.
#[inline]
fn check_constant_register_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[AbiType::Handle, AbiType::Handle, AbiType::Pointer],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Constant.Resolve` ABI function.
#[inline]
fn check_constant_resolve_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[AbiType::Handle, AbiType::Pointer, AbiType::Pointer],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Constant.IsRegistered` ABI function.
#[inline]
fn check_constant_is_registered_signature(signature: &Signature) -> bool {
    check_signature(signature, &[AbiType::Handle], &Some(AbiType::Boolean))
}

/// Checks the signature of the `Term.Register.Variable` ABI function.
#[inline]
fn check_term_register_variable_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[AbiType::Name, AbiType::Handle, AbiType::Pointer],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Term.Register.Constant` ABI function.
#[inline]
fn check_term_register_constant_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[
            AbiType::Handle,
            AbiType::Pointer,
            AbiType::Size,
            AbiType::Pointer,
            AbiType::Size,
            AbiType::Pointer,
        ],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Term.Register.Application` ABI function.
#[inline]
fn check_term_register_application_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[AbiType::Handle, AbiType::Handle, AbiType::Pointer],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Term.Register.Lambda` ABI function.
#[inline]
fn check_term_register_lambda_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[
            AbiType::Name,
            AbiType::Handle,
            AbiType::Handle,
            AbiType::Pointer,
        ],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Term.Register.Negation` ABI function.
#[inline]
fn check_term_register_negation_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[AbiType::Handle, AbiType::Pointer],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Term.Register.Conjunction` ABI function.
#[inline]
fn check_term_register_conjunction_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[AbiType::Handle, AbiType::Handle, AbiType::Pointer],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Term.Register.Disjunction` ABI function.
#[inline]
fn check_term_register_disjunction_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[AbiType::Handle, AbiType::Handle, AbiType::Pointer],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Term.Register.Implication` ABI function.
#[inline]
fn check_term_register_implication_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[AbiType::Handle, AbiType::Handle, AbiType::Pointer],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Term.Register.Equality` ABI function.
#[inline]
fn check_term_register_equality_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[AbiType::Handle, AbiType::Handle, AbiType::Pointer],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Term.Register.Forall` ABI function.
#[inline]
fn check_term_register_forall_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[
            AbiType::Name,
            AbiType::Handle,
            AbiType::Handle,
            AbiType::Pointer,
        ],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Term.Register.Exists` ABI function.
#[inline]
fn check_term_register_exists_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[
            AbiType::Name,
            AbiType::Handle,
            AbiType::Handle,
            AbiType::Pointer,
        ],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Term.Split.Variable` ABI function.
#[inline]
fn check_term_split_variable_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[AbiType::Handle, AbiType::Pointer, AbiType::Pointer],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Term.Split.Constant` ABI function.
#[inline]
fn check_term_split_constant_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[AbiType::Handle, AbiType::Pointer, AbiType::Pointer],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Term.Split.Application` ABI function.
#[inline]
fn check_term_split_application_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[AbiType::Handle, AbiType::Pointer, AbiType::Pointer],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Term.Split.Lambda` ABI function.
#[inline]
fn check_term_split_lambda_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[
            AbiType::Handle,
            AbiType::Pointer,
            AbiType::Pointer,
            AbiType::Pointer,
        ],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Term.Split.Negation` ABI function.
#[inline]
fn check_term_split_negation_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[AbiType::Handle, AbiType::Pointer],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Term.Split.Conjunction` ABI function.
#[inline]
fn check_term_split_conjunction_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[AbiType::Handle, AbiType::Pointer, AbiType::Pointer],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Term.Split.Disjunction` ABI function.
#[inline]
fn check_term_split_disjunction_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[AbiType::Handle, AbiType::Pointer, AbiType::Pointer],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Term.Split.Implication` ABI function.
#[inline]
fn check_term_split_implication_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[AbiType::Handle, AbiType::Pointer, AbiType::Pointer],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Term.Split.Equality` ABI function.
#[inline]
fn check_term_split_equality_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[AbiType::Handle, AbiType::Pointer, AbiType::Pointer],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Term.Split.Forall` ABI function.
#[inline]
fn check_term_split_forall_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[
            AbiType::Handle,
            AbiType::Pointer,
            AbiType::Pointer,
            AbiType::Pointer,
        ],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Term.Split.Exists` ABI function.
#[inline]
fn check_term_split_exists_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[
            AbiType::Handle,
            AbiType::Pointer,
            AbiType::Pointer,
            AbiType::Pointer,
        ],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Term.Test.Variable` ABI function.
#[inline]
fn check_term_test_variable_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[AbiType::Handle, AbiType::Pointer],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Term.Test.Constant` ABI function.
#[inline]
fn check_term_test_constant_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[AbiType::Handle, AbiType::Pointer],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Term.Test.Application` ABI function.
#[inline]
fn check_term_test_application_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[AbiType::Handle, AbiType::Pointer],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Term.Test.Lambda` ABI function.
#[inline]
fn check_term_test_lambda_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[AbiType::Handle, AbiType::Pointer],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Term.Test.Negation` ABI function.
#[inline]
fn check_term_test_negation_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[AbiType::Handle, AbiType::Pointer],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Term.Test.Conjunction` ABI function.
#[inline]
fn check_term_test_conjunction_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[AbiType::Handle, AbiType::Pointer],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Term.Test.Disjunction` ABI function.
#[inline]
fn check_term_test_disjunction_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[AbiType::Handle, AbiType::Pointer],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Term.Test.Implication` ABI function.
#[inline]
fn check_term_test_implication_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[AbiType::Handle, AbiType::Pointer],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Term.Test.Equality` ABI function.
#[inline]
fn check_term_test_equality_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[AbiType::Handle, AbiType::Pointer],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Term.Test.Forall` ABI function.
#[inline]
fn check_term_test_forall_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[AbiType::Handle, AbiType::Pointer],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Term.Test.Exists` ABI function.
#[inline]
fn check_term_test_exists_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[AbiType::Handle, AbiType::Pointer],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Term.FV` ABI function.
#[inline]
fn check_term_fv_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[
            AbiType::Handle,
            AbiType::Pointer,
            AbiType::Size,
            AbiType::Pointer,
            AbiType::Size,
        ],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Term.Substitution` ABI function.
#[inline]
fn check_term_substitution_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[
            AbiType::Handle,
            AbiType::Pointer,
            AbiType::Size,
            AbiType::Pointer,
            AbiType::Size,
            AbiType::Pointer,
        ],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Term.Type.Variables` ABI function.
#[inline]
fn check_term_type_variables_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[AbiType::Handle, AbiType::Pointer, AbiType::Size],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Term.Type.Substitution` ABI function.
#[inline]
fn check_term_type_substitution_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[
            AbiType::Handle,
            AbiType::Pointer,
            AbiType::Size,
            AbiType::Pointer,
        ],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Term.Type.Infer` ABI function.
#[inline]
fn check_term_type_infer_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[AbiType::Handle, AbiType::Pointer],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Term.Type.IsProposition` ABI function.
#[inline]
fn check_term_type_is_proposition_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[AbiType::Handle, AbiType::Pointer],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Theorem.IsRegistered` ABI function.
#[inline]
fn check_theorem_is_registered_signature(signature: &Signature) -> bool {
    check_signature(signature, &[AbiType::Handle], &Some(AbiType::ErrorCode))
}

/// Checks the signature of the `Theorem.Register.Assumption` ABI function.
#[inline]
fn check_theorem_register_assumption_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[AbiType::Handle, AbiType::Pointer],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Theorem.Register.Reflexivity` ABI function.
#[inline]
fn check_theorem_register_reflexivity_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[
            AbiType::Handle,
            AbiType::Pointer,
            AbiType::Size,
            AbiType::Pointer,
        ],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Theorem.Register.Symmetry` ABI function.
#[inline]
fn check_theorem_register_symmetry_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[AbiType::Handle, AbiType::Pointer],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Theorem.Register.Transitivity` ABI function.
#[inline]
fn check_theorem_register_transitivity_signature(
    signature: &Signature,
) -> bool {
    check_signature(
        signature,
        &[AbiType::Handle, AbiType::Handle, AbiType::Pointer],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Theorem.Register.Application` ABI function.
#[inline]
fn check_theorem_register_application_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[AbiType::Handle, AbiType::Handle, AbiType::Pointer],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Theorem.Register.Lambda` ABI function.
#[inline]
fn check_theorem_register_lambda_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[
            AbiType::Name,
            AbiType::Handle,
            AbiType::Handle,
            AbiType::Pointer,
        ],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Theorem.Register.Beta` ABI function.
#[inline]
fn check_theorem_register_beta_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[
            AbiType::Handle,
            AbiType::Pointer,
            AbiType::Size,
            AbiType::Pointer,
        ],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Theorem.Register.Eta` ABI function.
#[inline]
fn check_theorem_register_eta_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[
            AbiType::Handle,
            AbiType::Pointer,
            AbiType::Size,
            AbiType::Pointer,
        ],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Theorem.Register.TruthIntroduction` ABI function.
#[inline]
fn check_theorem_register_truth_introduction_signature(
    signature: &Signature,
) -> bool {
    check_signature(
        signature,
        &[AbiType::Pointer, AbiType::Size, AbiType::Pointer],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Theorem.Register.FalsityElimination` ABI function.
#[inline]
fn check_theorem_register_falsity_elimination_signature(
    signature: &Signature,
) -> bool {
    check_signature(
        signature,
        &[AbiType::Handle, AbiType::Pointer],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Theorem.Register.ConjunctionIntroduction` ABI function.
#[inline]
fn check_theorem_register_conjunction_introduction_signature(
    signature: &Signature,
) -> bool {
    check_signature(
        signature,
        &[AbiType::Handle, AbiType::Handle, AbiType::Pointer],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Theorem.Register.ConjunctionLeftElimination` ABI function.
#[inline]
fn check_theorem_register_conjunction_left_elimination_signature(
    signature: &Signature,
) -> bool {
    check_signature(
        signature,
        &[AbiType::Handle, AbiType::Pointer],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Theorem.Register.ConjunctionRightElimination` ABI function.
#[inline]
fn check_theorem_register_conjunction_right_elimination_signature(
    signature: &Signature,
) -> bool {
    check_signature(
        signature,
        &[AbiType::Handle, AbiType::Pointer],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Theorem.Register.DisjunctionElimination` ABI function.
#[inline]
fn check_theorem_register_disjunction_elimination_signature(
    signature: &Signature,
) -> bool {
    check_signature(
        signature,
        &[
            AbiType::Handle,
            AbiType::Handle,
            AbiType::Handle,
            AbiType::Handle,
            AbiType::Pointer,
        ],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Theorem.Register.DisjunctionLeftIntroduction` ABI function.
#[inline]
fn check_theorem_register_disjunction_left_introduction_signature(
    signature: &Signature,
) -> bool {
    check_signature(
        signature,
        &[AbiType::Handle, AbiType::Handle, AbiType::Pointer],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Theorem.Register.DisjunctionRightIntroduction` ABI function.
#[inline]
fn check_theorem_register_disjunction_right_introduction_signature(
    signature: &Signature,
) -> bool {
    check_signature(
        signature,
        &[AbiType::Handle, AbiType::Handle, AbiType::Pointer],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Theorem.Register.ImplicationIntroduction` ABI function.
#[inline]
fn check_theorem_register_implication_introduction_signature(
    signature: &Signature,
) -> bool {
    check_signature(
        signature,
        &[AbiType::Handle, AbiType::Handle, AbiType::Pointer],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Theorem.Register.ImplicationElimination` ABI function.
#[inline]
fn check_theorem_register_implication_elimination_signature(
    signature: &Signature,
) -> bool {
    check_signature(
        signature,
        &[AbiType::Handle, AbiType::Handle, AbiType::Pointer],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Theorem.Register.IffIntroduction` ABI function.
#[inline]
fn check_theorem_register_iff_introduction_signature(
    signature: &Signature,
) -> bool {
    check_signature(
        signature,
        &[AbiType::Handle, AbiType::Handle, AbiType::Pointer],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Theorem.Register.IffLeftElimination` ABI function.
#[inline]
fn check_theorem_register_iff_left_elimination_signature(
    signature: &Signature,
) -> bool {
    check_signature(
        signature,
        &[AbiType::Handle, AbiType::Handle, AbiType::Pointer],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Theorem.Register.NegationIntroduction` ABI function.
#[inline]
fn check_theorem_register_negation_introduction_signature(
    signature: &Signature,
) -> bool {
    check_signature(
        signature,
        &[AbiType::Handle, AbiType::Handle, AbiType::Pointer],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Theorem.Register.NegationElimination` ABI function.
#[inline]
fn check_theorem_register_negation_elimination_signature(
    signature: &Signature,
) -> bool {
    check_signature(
        signature,
        &[AbiType::Handle, AbiType::Handle, AbiType::Pointer],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Theorem.Register.ForallIntroduction` ABI function.
#[inline]
fn check_theorem_register_forall_introduction_signature(
    signature: &Signature,
) -> bool {
    check_signature(
        signature,
        &[
            AbiType::Name,
            AbiType::Handle,
            AbiType::Handle,
            AbiType::Pointer,
        ],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Theorem.Register.ForallElimination` ABI function.
#[inline]
fn check_theorem_register_forall_elimination_signature(
    signature: &Signature,
) -> bool {
    check_signature(
        signature,
        &[AbiType::Handle, AbiType::Handle, AbiType::Pointer],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Theorem.Register.ExistsIntroduction` ABI function.
#[inline]
fn check_theorem_register_exists_introduction_signature(
    _signature: &Signature,
) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Theorem.Register.ExistsElimination` ABI function.
#[inline]
fn check_theorem_register_exists_elimination_signature(
    _signature: &Signature,
) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Theorem.Register.Weaken` ABI function.
#[inline]
fn check_theorem_register_weaken_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[AbiType::Handle, AbiType::Pointer],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Theorem.Split.Conclusion` ABI function.
#[inline]
fn check_theorem_split_conclusion_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[AbiType::Handle, AbiType::Pointer],
        &Some(AbiType::ErrorCode),
    )
}

/// Checks the signature of the `Theorem.Split.Hypotheses` ABI function.
#[inline]
fn check_theorem_split_hypotheses_signature(signature: &Signature) -> bool {
    check_signature(
        signature,
        &[AbiType::Handle, AbiType::Pointer, AbiType::Size],
        &Some(AbiType::ErrorCode),
    )
}

////////////////////////////////////////////////////////////////////////////////
// ABI binding.
////////////////////////////////////////////////////////////////////////////////

/// Dispatches on an ABI host-call number, and calls the respective function on
/// the machine's runtime state.
impl Externals for WasmiRuntimeState {
    fn invoke_index(
        &mut self,
        index: usize,
        args: RuntimeArgs,
    ) -> Result<Option<RuntimeValue>, Trap> {
        match index {
            ABI_TYPE_FORMER_RESOLVE_INDEX => {
                let handle = args.nth::<semantic_types::Handle>(0);
                let result_addr = args.nth::<semantic_types::Pointer>(1);

                let arity = match self
                    .type_former_resolve(&Handle::from(handle as usize))
                {
                    None => {
                        return Ok(Some(RuntimeValue::I32(
                            KernelErrorCode::NoSuchTypeFormerRegistered.into(),
                        )))
                    }
                    Some(arity) => arity.clone(),
                };

                self.write_u64(result_addr, arity as u64)?;

                Ok(Some(RuntimeValue::I32(KernelErrorCode::Success.into())))
            }
            ABI_TYPE_FORMER_IS_REGISTERED_INDEX => {
                let handle = args.nth::<semantic_types::Handle>(0);
                let result = self
                    .type_former_is_registered(&Handle::from(handle as usize));

                Ok(Some(RuntimeValue::I32(result.into())))
            }
            ABI_TYPE_FORMER_REGISTER_INDEX => {
                let arity = args.nth::<semantic_types::Arity>(0);
                let result = self.type_former_register(arity as usize);

                Ok(Some(RuntimeValue::I32(*result as i32)))
            }
            ABI_TYPE_REGISTER_VARIABLE_INDEX => {
                let name = args.nth::<semantic_types::Name>(0);
                let result = self.type_register_variable(name);

                Ok(Some(RuntimeValue::I32(*result as i32)))
            }
            ABI_TYPE_REGISTER_COMBINATION_INDEX => {
                let former_handle: Handle<tags::TypeFormer> = Handle::from(
                    args.nth::<semantic_types::Handle>(0) as usize,
                );
                let argument_base = args.nth::<semantic_types::Pointer>(1);
                let argument_length = args.nth::<semantic_types::Size>(2);
                let result_ptr = args.nth::<semantic_types::Pointer>(3);

                let arguments =
                    self.read_handles(argument_base, argument_length as usize)?;

                match self.type_register_combination(former_handle, arguments) {
                    Err(e) => Ok(Some(RuntimeValue::I32(e as i32))),
                    Ok(result) => {
                        self.write_handle(result_ptr, result)?;

                        Ok(Some(RuntimeValue::I32(
                            KernelErrorCode::Success.into(),
                        )))
                    }
                }
            }
            ABI_TYPE_REGISTER_FUNCTION_INDEX => {
                let domain_handle: Handle<tags::Type> = Handle::from(
                    args.nth::<semantic_types::Handle>(0) as usize,
                );
                let range_handle: Handle<tags::Type> = Handle::from(
                    args.nth::<semantic_types::Handle>(1) as usize,
                );
                let result_ptr = args.nth::<semantic_types::Pointer>(2);

                match self.type_register_function(domain_handle, range_handle) {
                    Err(e) => Ok(Some(RuntimeValue::I32(e as i32))),
                    Ok(result) => {
                        self.write_handle(result_ptr, result)?;

                        Ok(Some(RuntimeValue::I32(
                            KernelErrorCode::Success.into(),
                        )))
                    }
                }
            }
            ABI_TYPE_IS_REGISTERED_INDEX => {
                let type_handle: Handle<tags::Type> = Handle::from(
                    args.nth::<semantic_types::Handle>(0) as usize,
                );
                Ok(Some(RuntimeValue::I32(
                    self.type_is_registered(type_handle).into(),
                )))
            }
            ABI_TYPE_SPLIT_VARIABLE_INDEX => {
                let type_handle: Handle<tags::Type> = Handle::from(
                    args.nth::<semantic_types::Handle>(0) as usize,
                );
                let result_ptr = args.nth::<semantic_types::Pointer>(1);

                match self.type_split_variable(type_handle) {
                    Err(e) => Ok(Some(RuntimeValue::I32(e as i32))),
                    Ok(result) => {
                        self.write_u64(result_ptr, result)?;

                        Ok(Some(RuntimeValue::I32(
                            KernelErrorCode::Success.into(),
                        )))
                    }
                }
            }
            ABI_TYPE_SPLIT_COMBINATION_INDEX => {
                let type_handle: Handle<tags::Type> = Handle::from(
                    args.nth::<semantic_types::Handle>(0) as usize,
                );
                let former_result_ptr = args.nth::<semantic_types::Pointer>(1);
                let arguments_result_ptr =
                    args.nth::<semantic_types::Pointer>(2);
                let arguments_length_result_ptr =
                    args.nth::<semantic_types::Pointer>(3);

                match self.type_split_combination(type_handle) {
                    Err(e) => Ok(Some(RuntimeValue::I32(e as i32))),
                    Ok((former_handle, arguments)) => {
                        self.write_handle(
                            former_result_ptr,
                            former_handle.clone(),
                        )?;
                        self.write_handles(
                            arguments_result_ptr,
                            arguments.clone(),
                        )?;
                        self.write_u64(
                            arguments_length_result_ptr,
                            arguments.len() as u64,
                        )?;

                        Ok(Some(RuntimeValue::I32(
                            KernelErrorCode::Success.into(),
                        )))
                    }
                }
            }
            ABI_TYPE_SPLIT_FUNCTION_INDEX => {
                let type_handle: Handle<tags::Type> = Handle::from(
                    args.nth::<semantic_types::Pointer>(0) as usize,
                );
                let domain_result_ptr = args.nth::<semantic_types::Pointer>(1);
                let range_result_ptr = args.nth::<semantic_types::Pointer>(2);

                match self.type_split_function(type_handle) {
                    Err(e) => Ok(Some(RuntimeValue::I32(e as i32))),
                    Ok((domain_handle, range_handle)) => {
                        self.write_handle(
                            domain_result_ptr,
                            domain_handle.clone(),
                        )?;
                        self.write_handle(
                            range_result_ptr,
                            range_handle.clone(),
                        )?;

                        Ok(Some(RuntimeValue::I32(
                            KernelErrorCode::Success.into(),
                        )))
                    }
                }
            }
            ABI_TYPE_TEST_VARIABLE_INDEX => {
                let type_handle: Handle<tags::Type> = Handle::from(
                    args.nth::<semantic_types::Handle>(0) as usize,
                );
                let result_ptr = args.nth::<semantic_types::Pointer>(1);

                match self.type_test_variable(type_handle) {
                    Err(e) => Ok(Some(RuntimeValue::I32(e as i32))),
                    Ok(result) => {
                        self.write_bool(result_ptr, result)?;

                        Ok(Some(RuntimeValue::I32(
                            KernelErrorCode::Success.into(),
                        )))
                    }
                }
            }
            ABI_TYPE_TEST_COMBINATION_INDEX => {
                let type_handle: Handle<tags::Type> = Handle::from(
                    args.nth::<semantic_types::Handle>(0) as usize,
                );
                let result_ptr = args.nth::<semantic_types::Pointer>(1);

                match self.type_test_combination(type_handle) {
                    Err(e) => Ok(Some(RuntimeValue::I32(e as i32))),
                    Ok(result) => {
                        self.write_bool(result_ptr, result)?;

                        Ok(Some(RuntimeValue::I32(
                            KernelErrorCode::Success.into(),
                        )))
                    }
                }
            }
            ABI_TYPE_TEST_FUNCTION_INDEX => {
                let type_handle: Handle<tags::Type> = Handle::from(
                    args.nth::<semantic_types::Handle>(0) as usize,
                );
                let result_ptr = args.nth::<semantic_types::Pointer>(1);

                match self.type_test_function(type_handle) {
                    Err(e) => Ok(Some(RuntimeValue::I32(e as i32))),
                    Ok(result) => {
                        self.write_bool(result_ptr, result)?;

                        Ok(Some(RuntimeValue::I32(
                            KernelErrorCode::Success.into(),
                        )))
                    }
                }
            }
            ABI_TYPE_VARIABLES_INDEX => {
                let type_handle: Handle<tags::Type> = Handle::from(
                    args.nth::<semantic_types::Handle>(0) as usize,
                );
                let variable_result_ptr =
                    args.nth::<semantic_types::Pointer>(1);
                let variable_len_ptr = args.nth::<semantic_types::Pointer>(2);

                match self.type_variables(type_handle) {
                    Err(e) => Ok(Some(RuntimeValue::I32(e as i32))),
                    Ok(result) => {
                        self.write_u64s(variable_result_ptr, result.clone())?;
                        self.write_u64(variable_len_ptr, result.len() as u64)?;

                        Ok(Some(RuntimeValue::I32(
                            KernelErrorCode::Success.into(),
                        )))
                    }
                }
            }
            ABI_TYPE_SUBSTITUTE_INDEX => {
                let type_handle: Handle<tags::Type> = Handle::from(
                    args.nth::<semantic_types::Handle>(0) as usize,
                );
                let dom_ptr = args.nth::<semantic_types::Pointer>(1);
                let dom_len = args.nth::<semantic_types::Size>(2);
                let rng_ptr = args.nth::<semantic_types::Pointer>(3);
                let rng_len = args.nth::<semantic_types::Size>(4);
                let result_ptr = args.nth::<semantic_types::Pointer>(5);

                let doms = self.read_u64s(dom_ptr, dom_len as usize)?;
                let rngs = self.read_handles(rng_ptr, rng_len as usize)?;

                let subst = doms
                    .iter()
                    .zip(rngs)
                    .map(|(d, r)| (d.clone(), r.clone()))
                    .collect();

                match self.type_substitute(type_handle, subst) {
                    Err(e) => Ok(Some(RuntimeValue::I32(e as i32))),
                    Ok(result) => {
                        self.write_handle(result_ptr, result)?;

                        Ok(Some(RuntimeValue::I32(
                            KernelErrorCode::Success.into(),
                        )))
                    }
                }
            }
            ABI_CONSTANT_REGISTER_INDEX => {
                let type_handle: Handle<tags::Type> = Handle::from(
                    args.nth::<semantic_types::Handle>(0) as usize,
                );
                let result_ptr = args.nth::<semantic_types::Pointer>(1);

                match self.constant_register(type_handle) {
                    Err(e) => Ok(Some(RuntimeValue::I32(e as i32))),
                    Ok(result) => {
                        self.write_handle(result_ptr, result)?;

                        Ok(Some(RuntimeValue::I32(
                            KernelErrorCode::Success.into(),
                        )))
                    }
                }
            }
            ABI_CONSTANT_IS_REGISTERED_INDEX => {
                let constant_handle: Handle<tags::Constant> = Handle::from(
                    args.nth::<semantic_types::Handle>(0) as usize,
                );

                Ok(Some(RuntimeValue::I32(
                    self.constant_is_registered(constant_handle).into(),
                )))
            }
            ABI_CONSTANT_RESOLVE_INDEX => {
                let constant_handle: Handle<tags::Constant> = Handle::from(
                    args.nth::<semantic_types::Handle>(0) as usize,
                );
                let result_ptr = args.nth::<semantic_types::Pointer>(1);

                match self.constant_resolve(constant_handle) {
                    Err(e) => Ok(Some(RuntimeValue::I32(e as i32))),
                    Ok(result) => {
                        self.write_handle(result_ptr, result)?;

                        Ok(Some(RuntimeValue::I32(
                            KernelErrorCode::Success.into(),
                        )))
                    }
                }
            }
            ABI_TERM_REGISTER_VARIABLE_INDEX => {
                let name = args.nth::<semantic_types::Name>(0);
                let type_handle: Handle<tags::Type> = Handle::from(
                    args.nth::<semantic_types::Handle>(1) as usize,
                );
                let result_ptr = args.nth::<semantic_types::Pointer>(2);

                match self.term_register_variable(name, type_handle) {
                    Err(e) => Ok(Some(RuntimeValue::I32(e as i32))),
                    Ok(result) => {
                        self.write_handle(result_ptr, result)?;

                        Ok(Some(RuntimeValue::I32(
                            KernelErrorCode::Success.into(),
                        )))
                    }
                }
            }
            ABI_TERM_REGISTER_CONSTANT_INDEX => {
                let constant_handle: Handle<tags::Constant> = Handle::from(
                    args.nth::<semantic_types::Handle>(0) as usize,
                );
                let dom_ptr = args.nth::<semantic_types::Pointer>(1);
                let dom_len = args.nth::<semantic_types::Size>(2);
                let rng_ptr = args.nth::<semantic_types::Pointer>(3);
                let rng_len = args.nth::<semantic_types::Size>(4);
                let result_ptr = args.nth::<semantic_types::Pointer>(5);

                let doms = self.read_u64s(dom_ptr, dom_len as usize)?;
                let rngs = self.read_handles(rng_ptr, rng_len as usize)?;

                let subst = doms
                    .iter()
                    .zip(rngs)
                    .map(|(d, r)| (d.clone(), r.clone()))
                    .collect();

                match self.term_register_constant(constant_handle, subst) {
                    Err(e) => Ok(Some(RuntimeValue::I32(e as i32))),
                    Ok(result) => {
                        self.write_handle(result_ptr, result)?;

                        Ok(Some(RuntimeValue::I32(
                            KernelErrorCode::Success.into(),
                        )))
                    }
                }
            }
            ABI_TERM_REGISTER_APPLICATION_INDEX => {
                let left_handle: Handle<tags::Term> = Handle::from(
                    args.nth::<semantic_types::Handle>(0) as usize,
                );
                let right_handle: Handle<tags::Term> = Handle::from(
                    args.nth::<semantic_types::Handle>(1) as usize,
                );
                let result_ptr = args.nth::<semantic_types::Pointer>(2);

                match self.term_register_application(left_handle, right_handle)
                {
                    Err(e) => Ok(Some(RuntimeValue::I32(e as i32))),
                    Ok(result) => {
                        self.write_handle(result_ptr, result)?;

                        Ok(Some(RuntimeValue::I32(
                            KernelErrorCode::Success.into(),
                        )))
                    }
                }
            }
            ABI_TERM_REGISTER_LAMBDA_INDEX => {
                let name = args.nth::<semantic_types::Name>(0);
                let type_handle: Handle<tags::Type> = Handle::from(
                    args.nth::<semantic_types::Handle>(1) as usize,
                );
                let body_handle: Handle<tags::Term> = Handle::from(
                    args.nth::<semantic_types::Handle>(2) as usize,
                );
                let result_ptr = args.nth::<semantic_types::Pointer>(3);

                match self.term_register_lambda(name, type_handle, body_handle)
                {
                    Err(e) => Ok(Some(RuntimeValue::I32(e as i32))),
                    Ok(result) => {
                        self.write_handle(result_ptr, result)?;

                        Ok(Some(RuntimeValue::I32(
                            KernelErrorCode::Success.into(),
                        )))
                    }
                }
            }
            ABI_TERM_REGISTER_NEGATION_INDEX => {
                let term_handle: Handle<tags::Term> = Handle::from(
                    args.nth::<semantic_types::Handle>(0) as usize,
                );
                let result_ptr = args.nth::<semantic_types::Pointer>(1);

                match self.term_register_negation(term_handle) {
                    Err(e) => Ok(Some(RuntimeValue::I32(e as i32))),
                    Ok(result) => {
                        self.write_handle(result_ptr, result)?;

                        Ok(Some(RuntimeValue::I32(
                            KernelErrorCode::Success.into(),
                        )))
                    }
                }
            }
            ABI_TERM_REGISTER_CONJUNCTION_INDEX => {
                let left_handle: Handle<tags::Term> = Handle::from(
                    args.nth::<semantic_types::Handle>(0) as usize,
                );
                let right_handle: Handle<tags::Term> = Handle::from(
                    args.nth::<semantic_types::Handle>(1) as usize,
                );
                let result_ptr = args.nth::<semantic_types::Pointer>(2);

                match self.term_register_conjunction(left_handle, right_handle)
                {
                    Err(e) => Ok(Some(RuntimeValue::I32(e as i32))),
                    Ok(result) => {
                        self.write_handle(result_ptr, result)?;

                        Ok(Some(RuntimeValue::I32(
                            KernelErrorCode::Success.into(),
                        )))
                    }
                }
            }
            ABI_TERM_REGISTER_DISJUNCTION_INDEX => {
                let left_handle: Handle<tags::Term> = Handle::from(
                    args.nth::<semantic_types::Handle>(0) as usize,
                );
                let right_handle: Handle<tags::Term> = Handle::from(
                    args.nth::<semantic_types::Handle>(1) as usize,
                );
                let result_ptr = args.nth::<semantic_types::Pointer>(2);

                match self.term_register_disjunction(left_handle, right_handle)
                {
                    Err(e) => Ok(Some(RuntimeValue::I32(e as i32))),
                    Ok(result) => {
                        self.write_handle(result_ptr, result)?;

                        Ok(Some(RuntimeValue::I32(
                            KernelErrorCode::Success.into(),
                        )))
                    }
                }
            }
            ABI_TERM_REGISTER_IMPLICATION_INDEX => {
                let left_handle: Handle<tags::Term> = Handle::from(
                    args.nth::<semantic_types::Handle>(0) as usize,
                );
                let right_handle: Handle<tags::Term> = Handle::from(
                    args.nth::<semantic_types::Handle>(1) as usize,
                );
                let result_ptr = args.nth::<semantic_types::Pointer>(2);

                match self.term_register_implication(left_handle, right_handle)
                {
                    Err(e) => Ok(Some(RuntimeValue::I32(e as i32))),
                    Ok(result) => {
                        self.write_handle(result_ptr, result)?;

                        Ok(Some(RuntimeValue::I32(
                            KernelErrorCode::Success.into(),
                        )))
                    }
                }
            }
            ABI_TERM_REGISTER_EQUALITY_INDEX => {
                let left_handle: Handle<tags::Term> = Handle::from(
                    args.nth::<semantic_types::Handle>(0) as usize,
                );
                let right_handle: Handle<tags::Term> = Handle::from(
                    args.nth::<semantic_types::Handle>(1) as usize,
                );
                let result_ptr = args.nth::<semantic_types::Pointer>(2);

                match self.term_register_equality(left_handle, right_handle) {
                    Err(e) => Ok(Some(RuntimeValue::I32(e as i32))),
                    Ok(result) => {
                        self.write_handle(result_ptr, result)?;

                        Ok(Some(RuntimeValue::I32(
                            KernelErrorCode::Success.into(),
                        )))
                    }
                }
            }
            ABI_TERM_REGISTER_FORALL_INDEX => {
                let name = args.nth::<semantic_types::Name>(0);
                let type_handle: Handle<tags::Type> = Handle::from(
                    args.nth::<semantic_types::Handle>(1) as usize,
                );
                let body_handle: Handle<tags::Term> = Handle::from(
                    args.nth::<semantic_types::Handle>(2) as usize,
                );
                let result_ptr = args.nth::<semantic_types::Pointer>(3);

                match self.term_register_forall(name, type_handle, body_handle)
                {
                    Err(e) => Ok(Some(RuntimeValue::I32(e as i32))),
                    Ok(result) => {
                        self.write_handle(result_ptr, result)?;

                        Ok(Some(RuntimeValue::I32(
                            KernelErrorCode::Success.into(),
                        )))
                    }
                }
            }
            ABI_TERM_REGISTER_EXISTS_INDEX => {
                let name = args.nth::<semantic_types::Name>(0);
                let type_handle: Handle<tags::Type> = Handle::from(
                    args.nth::<semantic_types::Handle>(1) as usize,
                );
                let body_handle: Handle<tags::Term> = Handle::from(
                    args.nth::<semantic_types::Handle>(2) as usize,
                );
                let result_ptr = args.nth::<semantic_types::Pointer>(3);

                match self.term_register_exists(name, type_handle, body_handle)
                {
                    Err(e) => Ok(Some(RuntimeValue::I32(e as i32))),
                    Ok(result) => {
                        self.write_handle(result_ptr, result)?;

                        Ok(Some(RuntimeValue::I32(
                            KernelErrorCode::Success.into(),
                        )))
                    }
                }
            }
            ABI_TERM_SPLIT_VARIABLE_INDEX => {
                let term_handle: Handle<tags::Term> = Handle::from(
                    args.nth::<semantic_types::Handle>(0) as usize,
                );
                let result_name_ptr = args.nth::<semantic_types::Pointer>(1);
                let result_type_ptr = args.nth::<semantic_types::Pointer>(2);

                match self.term_split_variable(term_handle) {
                    Err(e) => Ok(Some(RuntimeValue::I32(e as i32))),
                    Ok((name, tau)) => {
                        self.write_u64(result_name_ptr, name)?;
                        self.write_handle(result_type_ptr, tau)?;

                        Ok(Some(RuntimeValue::I32(
                            KernelErrorCode::Success.into(),
                        )))
                    }
                }
            }
            ABI_TERM_SPLIT_CONSTANT_INDEX => {
                let term_handle: Handle<tags::Term> = Handle::from(
                    args.nth::<semantic_types::Handle>(0) as usize,
                );
                let result_const_ptr = args.nth::<semantic_types::Pointer>(1);
                let result_type_ptr = args.nth::<semantic_types::Pointer>(2);

                match self.term_split_constant(term_handle) {
                    Err(e) => Ok(Some(RuntimeValue::I32(e as i32))),
                    Ok((constant, tau)) => {
                        self.write_handle(result_const_ptr, constant)?;
                        self.write_handle(result_type_ptr, tau)?;

                        Ok(Some(RuntimeValue::I32(
                            KernelErrorCode::Success.into(),
                        )))
                    }
                }
            }
            ABI_TERM_SPLIT_APPLICATION_INDEX => {
                let term_handle: Handle<tags::Term> = Handle::from(
                    args.nth::<semantic_types::Handle>(0) as usize,
                );
                let result_left_ptr = args.nth::<semantic_types::Pointer>(1);
                let result_right_ptr = args.nth::<semantic_types::Pointer>(2);

                match self.term_split_application(term_handle) {
                    Err(e) => Ok(Some(RuntimeValue::I32(e as i32))),
                    Ok((left, right)) => {
                        self.write_handle(result_left_ptr, left)?;
                        self.write_handle(result_right_ptr, right)?;

                        Ok(Some(RuntimeValue::I32(
                            KernelErrorCode::Success.into(),
                        )))
                    }
                }
            }
            ABI_TERM_SPLIT_LAMBDA_INDEX => {
                let term_handle: Handle<tags::Term> = Handle::from(
                    args.nth::<semantic_types::Handle>(0) as usize,
                );
                let result_name_ptr = args.nth::<semantic_types::Pointer>(1);
                let result_type_ptr = args.nth::<semantic_types::Pointer>(2);
                let result_body_ptr = args.nth::<semantic_types::Pointer>(3);

                match self.term_split_lambda(term_handle) {
                    Err(e) => Ok(Some(RuntimeValue::I32(e as i32))),
                    Ok((name, tau, body)) => {
                        self.write_u64(result_name_ptr, name)?;
                        self.write_handle(result_type_ptr, tau)?;
                        self.write_handle(result_body_ptr, body)?;

                        Ok(Some(RuntimeValue::I32(
                            KernelErrorCode::Success.into(),
                        )))
                    }
                }
            }
            ABI_TERM_SPLIT_NEGATION_INDEX => {
                let term_handle: Handle<tags::Term> = Handle::from(
                    args.nth::<semantic_types::Handle>(0) as usize,
                );
                let result_body_ptr = args.nth::<semantic_types::Pointer>(1);

                match self.term_split_negation(term_handle) {
                    Err(e) => Ok(Some(RuntimeValue::I32(e as i32))),
                    Ok(body) => {
                        self.write_handle(result_body_ptr, body)?;

                        Ok(Some(RuntimeValue::I32(
                            KernelErrorCode::Success.into(),
                        )))
                    }
                }
            }
            ABI_TERM_SPLIT_CONJUNCTION_INDEX => {
                let term_handle: Handle<tags::Term> = Handle::from(
                    args.nth::<semantic_types::Handle>(0) as usize,
                );
                let result_left_ptr = args.nth::<semantic_types::Pointer>(1);
                let result_right_ptr = args.nth::<semantic_types::Pointer>(2);

                match self.term_split_conjunction(term_handle) {
                    Err(e) => Ok(Some(RuntimeValue::I32(e as i32))),
                    Ok((left, right)) => {
                        self.write_handle(result_left_ptr, left)?;
                        self.write_handle(result_right_ptr, right)?;

                        Ok(Some(RuntimeValue::I32(
                            KernelErrorCode::Success.into(),
                        )))
                    }
                }
            }
            ABI_TERM_SPLIT_DISJUNCTION_INDEX => {
                let term_handle: Handle<tags::Term> = Handle::from(
                    args.nth::<semantic_types::Handle>(0) as usize,
                );
                let result_left_ptr = args.nth::<semantic_types::Pointer>(1);
                let result_right_ptr = args.nth::<semantic_types::Pointer>(2);

                match self.term_split_disjunction(term_handle) {
                    Err(e) => Ok(Some(RuntimeValue::I32(e as i32))),
                    Ok((left, right)) => {
                        self.write_handle(result_left_ptr, left)?;
                        self.write_handle(result_right_ptr, right)?;

                        Ok(Some(RuntimeValue::I32(
                            KernelErrorCode::Success.into(),
                        )))
                    }
                }
            }
            ABI_TERM_SPLIT_IMPLICATION_INDEX => {
                let term_handle: Handle<tags::Term> = Handle::from(
                    args.nth::<semantic_types::Handle>(0) as usize,
                );
                let result_left_ptr = args.nth::<semantic_types::Pointer>(1);
                let result_right_ptr = args.nth::<semantic_types::Pointer>(2);

                match self.term_split_implication(term_handle) {
                    Err(e) => Ok(Some(RuntimeValue::I32(e as i32))),
                    Ok((left, right)) => {
                        self.write_handle(result_left_ptr, left)?;
                        self.write_handle(result_right_ptr, right)?;

                        Ok(Some(RuntimeValue::I32(
                            KernelErrorCode::Success.into(),
                        )))
                    }
                }
            }
            ABI_TERM_SPLIT_EQUALITY_INDEX => {
                let term_handle: Handle<tags::Term> = Handle::from(
                    args.nth::<semantic_types::Handle>(0) as usize,
                );
                let result_left_ptr = args.nth::<semantic_types::Pointer>(1);
                let result_right_ptr = args.nth::<semantic_types::Pointer>(2);

                match self.term_split_equality(term_handle) {
                    Err(e) => Ok(Some(RuntimeValue::I32(e as i32))),
                    Ok((left, right)) => {
                        self.write_handle(result_left_ptr, left)?;
                        self.write_handle(result_right_ptr, right)?;

                        Ok(Some(RuntimeValue::I32(
                            KernelErrorCode::Success.into(),
                        )))
                    }
                }
            }
            ABI_TERM_SPLIT_FORALL_INDEX => {
                let term_handle: Handle<tags::Term> = Handle::from(
                    args.nth::<semantic_types::Handle>(0) as usize,
                );
                let result_name_ptr = args.nth::<semantic_types::Pointer>(1);
                let result_type_ptr = args.nth::<semantic_types::Pointer>(2);
                let result_body_ptr = args.nth::<semantic_types::Pointer>(3);

                match self.term_split_forall(term_handle) {
                    Err(e) => Ok(Some(RuntimeValue::I32(e as i32))),
                    Ok((name, tau, body)) => {
                        self.write_u64(result_name_ptr, name)?;
                        self.write_handle(result_type_ptr, tau)?;
                        self.write_handle(result_body_ptr, body)?;

                        Ok(Some(RuntimeValue::I32(
                            KernelErrorCode::Success.into(),
                        )))
                    }
                }
            }
            ABI_TERM_SPLIT_EXISTS_INDEX => {
                let term_handle: Handle<tags::Term> = Handle::from(
                    args.nth::<semantic_types::Handle>(0) as usize,
                );
                let result_name_ptr = args.nth::<semantic_types::Pointer>(1);
                let result_type_ptr = args.nth::<semantic_types::Pointer>(2);
                let result_body_ptr = args.nth::<semantic_types::Pointer>(3);

                match self.term_split_exists(term_handle) {
                    Err(e) => Ok(Some(RuntimeValue::I32(e as i32))),
                    Ok((name, tau, body)) => {
                        self.write_u64(result_name_ptr, name)?;
                        self.write_handle(result_type_ptr, tau)?;
                        self.write_handle(result_body_ptr, body)?;

                        Ok(Some(RuntimeValue::I32(
                            KernelErrorCode::Success.into(),
                        )))
                    }
                }
            }
            ABI_TERM_TEST_VARIABLE_INDEX => {
                let term_handle: Handle<tags::Term> = Handle::from(
                    args.nth::<semantic_types::Handle>(0) as usize,
                );
                let result_ptr = args.nth::<semantic_types::Pointer>(1);

                match self.term_test_variable(term_handle) {
                    Err(e) => Ok(Some(RuntimeValue::I32(e as i32))),
                    Ok(result) => {
                        self.write_bool(result_ptr, result)?;

                        Ok(Some(RuntimeValue::I32(
                            KernelErrorCode::Success.into(),
                        )))
                    }
                }
            }
            ABI_TERM_TEST_CONSTANT_INDEX => {
                let term_handle: Handle<tags::Term> = Handle::from(
                    args.nth::<semantic_types::Handle>(0) as usize,
                );
                let result_ptr = args.nth::<semantic_types::Pointer>(1);

                match self.term_test_constant(term_handle) {
                    Err(e) => Ok(Some(RuntimeValue::I32(e as i32))),
                    Ok(result) => {
                        self.write_bool(result_ptr, result)?;

                        Ok(Some(RuntimeValue::I32(
                            KernelErrorCode::Success.into(),
                        )))
                    }
                }
            }
            ABI_TERM_TEST_APPLICATION_INDEX => {
                let term_handle: Handle<tags::Term> = Handle::from(
                    args.nth::<semantic_types::Handle>(0) as usize,
                );
                let result_ptr = args.nth::<semantic_types::Pointer>(1);

                match self.term_test_application(term_handle) {
                    Err(e) => Ok(Some(RuntimeValue::I32(e as i32))),
                    Ok(result) => {
                        self.write_bool(result_ptr, result)?;

                        Ok(Some(RuntimeValue::I32(
                            KernelErrorCode::Success.into(),
                        )))
                    }
                }
            }
            ABI_TERM_TEST_LAMBDA_INDEX => {
                let term_handle: Handle<tags::Term> = Handle::from(
                    args.nth::<semantic_types::Handle>(0) as usize,
                );
                let result_ptr = args.nth::<semantic_types::Pointer>(1);

                match self.term_test_lambda(term_handle) {
                    Err(e) => Ok(Some(RuntimeValue::I32(e as i32))),
                    Ok(result) => {
                        self.write_bool(result_ptr, result)?;

                        Ok(Some(RuntimeValue::I32(
                            KernelErrorCode::Success.into(),
                        )))
                    }
                }
            }
            ABI_TERM_TEST_NEGATION_INDEX => {
                let term_handle: Handle<tags::Term> = Handle::from(
                    args.nth::<semantic_types::Handle>(0) as usize,
                );
                let result_ptr = args.nth::<semantic_types::Pointer>(1);

                match self.term_test_negation(term_handle) {
                    Err(e) => Ok(Some(RuntimeValue::I32(e as i32))),
                    Ok(result) => {
                        self.write_bool(result_ptr, result)?;

                        Ok(Some(RuntimeValue::I32(
                            KernelErrorCode::Success.into(),
                        )))
                    }
                }
            }
            ABI_TERM_TEST_CONJUNCTION_INDEX => {
                let term_handle: Handle<tags::Term> = Handle::from(
                    args.nth::<semantic_types::Handle>(0) as usize,
                );
                let result_ptr = args.nth::<semantic_types::Pointer>(1);

                match self.term_test_conjunction(term_handle) {
                    Err(e) => Ok(Some(RuntimeValue::I32(e as i32))),
                    Ok(result) => {
                        self.write_bool(result_ptr, result)?;

                        Ok(Some(RuntimeValue::I32(
                            KernelErrorCode::Success.into(),
                        )))
                    }
                }
            }
            ABI_TERM_TEST_DISJUNCTION_INDEX => {
                let term_handle: Handle<tags::Term> = Handle::from(
                    args.nth::<semantic_types::Handle>(0) as usize,
                );
                let result_ptr = args.nth::<semantic_types::Pointer>(1);

                match self.term_test_disjunction(term_handle) {
                    Err(e) => Ok(Some(RuntimeValue::I32(e as i32))),
                    Ok(result) => {
                        self.write_bool(result_ptr, result)?;

                        Ok(Some(RuntimeValue::I32(
                            KernelErrorCode::Success.into(),
                        )))
                    }
                }
            }
            ABI_TERM_TEST_IMPLICATION_INDEX => {
                let term_handle: Handle<tags::Term> = Handle::from(
                    args.nth::<semantic_types::Handle>(0) as usize,
                );
                let result_ptr = args.nth::<semantic_types::Pointer>(1);

                match self.term_test_implication(term_handle) {
                    Err(e) => Ok(Some(RuntimeValue::I32(e as i32))),
                    Ok(result) => {
                        self.write_bool(result_ptr, result)?;

                        Ok(Some(RuntimeValue::I32(
                            KernelErrorCode::Success.into(),
                        )))
                    }
                }
            }
            ABI_TERM_TEST_EQUALITY_INDEX => {
                let term_handle: Handle<tags::Term> = Handle::from(
                    args.nth::<semantic_types::Handle>(0) as usize,
                );
                let result_ptr = args.nth::<semantic_types::Pointer>(1);

                match self.term_test_equality(term_handle) {
                    Err(e) => Ok(Some(RuntimeValue::I32(e as i32))),
                    Ok(result) => {
                        self.write_bool(result_ptr, result)?;

                        Ok(Some(RuntimeValue::I32(
                            KernelErrorCode::Success.into(),
                        )))
                    }
                }
            }
            ABI_TERM_TEST_FORALL_INDEX => {
                let term_handle: Handle<tags::Term> = Handle::from(
                    args.nth::<semantic_types::Handle>(0) as usize,
                );
                let result_ptr = args.nth::<semantic_types::Pointer>(1);

                match self.term_test_forall(term_handle) {
                    Err(e) => Ok(Some(RuntimeValue::I32(e as i32))),
                    Ok(result) => {
                        self.write_bool(result_ptr, result)?;

                        Ok(Some(RuntimeValue::I32(
                            KernelErrorCode::Success.into(),
                        )))
                    }
                }
            }
            ABI_TERM_TEST_EXISTS_INDEX => {
                let term_handle: Handle<tags::Term> = Handle::from(
                    args.nth::<semantic_types::Handle>(0) as usize,
                );
                let result_ptr = args.nth::<semantic_types::Pointer>(1);

                match self.term_test_exists(term_handle) {
                    Err(e) => Ok(Some(RuntimeValue::I32(e as i32))),
                    Ok(result) => {
                        self.write_bool(result_ptr, result)?;

                        Ok(Some(RuntimeValue::I32(
                            KernelErrorCode::Success.into(),
                        )))
                    }
                }
            }
            ABI_TERM_FREE_VARIABLES_INDEX => {
                let term_handle: Handle<tags::Term> = Handle::from(
                    args.nth::<semantic_types::Handle>(0) as usize,
                );
                let result_name_base_ptr =
                    args.nth::<semantic_types::Pointer>(1);
                let result_name_len_ptr =
                    args.nth::<semantic_types::Pointer>(2);
                let result_type_base_ptr =
                    args.nth::<semantic_types::Pointer>(1);
                let result_type_len_ptr =
                    args.nth::<semantic_types::Pointer>(2);

                match self.term_free_variables(term_handle) {
                    Err(e) => Ok(Some(RuntimeValue::I32(e as i32))),
                    Ok(result) => {
                        let (names, types): (
                            Vec<Name>,
                            Vec<Handle<tags::Type>>,
                        ) = result.iter().cloned().unzip();

                        self.write_u64(
                            result_name_len_ptr,
                            names.len() as u64,
                        )?;
                        self.write_u64s(result_name_base_ptr, names)?;
                        self.write_u64(
                            result_type_len_ptr,
                            types.len() as u64,
                        )?;
                        self.write_handles(result_type_base_ptr, types)?;

                        Ok(Some(RuntimeValue::I32(
                            KernelErrorCode::Success.into(),
                        )))
                    }
                }
            }
            ABI_TERM_SUBSTITUTION_INDEX => {
                let term_handle: Handle<tags::Term> = Handle::from(
                    args.nth::<semantic_types::Handle>(0) as usize,
                );
                let dom_ptr = args.nth::<semantic_types::Pointer>(1);
                let dom_len = args.nth::<semantic_types::Size>(2);
                let rng_ptr = args.nth::<semantic_types::Pointer>(3);
                let rng_len = args.nth::<semantic_types::Size>(4);
                let result_ptr = args.nth::<semantic_types::Pointer>(5);

                let doms = self.read_u64s(dom_ptr, dom_len as usize)?;
                let rngs = self.read_handles(rng_ptr, rng_len as usize)?;

                let subst = doms
                    .iter()
                    .zip(rngs)
                    .map(|(d, r)| (d.clone(), r.clone()))
                    .collect();

                match self.term_substitution(term_handle, subst) {
                    Err(e) => Ok(Some(RuntimeValue::I32(e as i32))),
                    Ok(result) => {
                        self.write_handle(result_ptr, result)?;

                        Ok(Some(RuntimeValue::I32(
                            KernelErrorCode::Success.into(),
                        )))
                    }
                }
            }
            ABI_TERM_TYPE_VARIABLES_INDEX => {
                let term_handle: Handle<tags::Term> = Handle::from(
                    args.nth::<semantic_types::Handle>(0) as usize,
                );
                let result_base_ptr = args.nth::<semantic_types::Pointer>(1);
                let result_len_ptr = args.nth::<semantic_types::Pointer>(2);

                match self.term_type_variables(term_handle) {
                    Err(e) => Ok(Some(RuntimeValue::I32(e as i32))),
                    Ok(result) => {
                        self.write_u64(result_len_ptr, result.len() as u64)?;
                        self.write_u64s(result_base_ptr, result)?;

                        Ok(Some(RuntimeValue::I32(
                            KernelErrorCode::Success.into(),
                        )))
                    }
                }
            }
            ABI_TERM_TYPE_SUBSTITUTION_INDEX => {
                let term_handle: Handle<tags::Term> = Handle::from(
                    args.nth::<semantic_types::Handle>(0) as usize,
                );

                let dom_ptr = args.nth::<semantic_types::Pointer>(1);
                let dom_len = args.nth::<semantic_types::Size>(2);
                let rng_ptr = args.nth::<semantic_types::Pointer>(3);
                let rng_len = args.nth::<semantic_types::Size>(4);
                let result_ptr = args.nth::<semantic_types::Pointer>(5);

                let doms = self.read_u64s(dom_ptr, dom_len as usize)?;
                let rngs = self.read_handles(rng_ptr, rng_len as usize)?;

                let subst = doms
                    .iter()
                    .zip(rngs)
                    .map(|(d, r)| (d.clone(), r.clone()))
                    .collect();

                match self.term_type_substitution(term_handle, subst) {
                    Err(e) => Ok(Some(RuntimeValue::I32(e as i32))),
                    Ok(result) => {
                        self.write_handle(result_ptr, result)?;

                        Ok(Some(RuntimeValue::I32(
                            KernelErrorCode::Success.into(),
                        )))
                    }
                }
            }
            ABI_TERM_TYPE_INFER_INDEX => {
                let term_handle: Handle<tags::Term> = Handle::from(
                    args.nth::<semantic_types::Handle>(0) as usize,
                );
                let result_ptr = args.nth::<semantic_types::Pointer>(1);

                match self.term_type_infer(term_handle) {
                    Err(e) => Ok(Some(RuntimeValue::I32(e as i32))),
                    Ok(result) => {
                        self.write_handle(result_ptr, result)?;

                        Ok(Some(RuntimeValue::I32(
                            KernelErrorCode::Success.into(),
                        )))
                    }
                }
            }
            ABI_TERM_TYPE_IS_PROPOSITION_INDEX => {
                let term_handle: Handle<tags::Term> = Handle::from(
                    args.nth::<semantic_types::Handle>(0) as usize,
                );
                let result_ptr = args.nth::<semantic_types::Pointer>(1);

                match self.term_type_is_proposition(term_handle) {
                    Err(e) => Ok(Some(RuntimeValue::I32(e as i32))),
                    Ok(result) => {
                        self.write_bool(result_ptr, result)?;

                        Ok(Some(RuntimeValue::I32(
                            KernelErrorCode::Success.into(),
                        )))
                    }
                }
            }
            ABI_THEOREM_IS_REGISTERED_INDEX => {
                let theorem_handle: Handle<tags::Theorem> = Handle::from(
                    args.nth::<semantic_types::Handle>(0) as usize,
                );
                let result = self.theorem_is_registered(theorem_handle);

                Ok(Some(RuntimeValue::I32(result.into())))
            }
            ABI_THEOREM_SPLIT_CONCLUSION_INDEX => {
                let theorem_handle: Handle<tags::Theorem> = Handle::from(
                    args.nth::<semantic_types::Handle>(0) as usize,
                );
                let result_ptr = args.nth::<semantic_types::Pointer>(1);

                match self.theorem_split_conclusion(theorem_handle) {
                    Err(e) => Ok(Some(RuntimeValue::I32(e as i32))),
                    Ok(result) => {
                        self.write_handle(result_ptr, result)?;

                        Ok(Some(RuntimeValue::I32(
                            KernelErrorCode::Success.into(),
                        )))
                    }
                }
            }
            ABI_THEOREM_SPLIT_HYPOTHESES_INDEX => {
                let theorem_handle: Handle<tags::Theorem> = Handle::from(
                    args.nth::<semantic_types::Handle>(0) as usize,
                );
                let result_base_ptr = args.nth::<semantic_types::Pointer>(1);
                let result_len_ptr = args.nth::<semantic_types::Pointer>(2);

                match self.theorem_split_hypotheses(theorem_handle) {
                    Err(e) => Ok(Some(RuntimeValue::I32(e as i32))),
                    Ok(result) => {
                        self.write_u64(result_len_ptr, result.len() as u64)?;
                        self.write_handles(result_base_ptr, result)?;

                        Ok(Some(RuntimeValue::I32(
                            KernelErrorCode::Success.into(),
                        )))
                    }
                }
            }
            ABI_THEOREM_REGISTER_ASSUMPTION_INDEX => unimplemented!(),
            ABI_THEOREM_REGISTER_REFLEXIVITY_INDEX => unimplemented!(),
            ABI_THEOREM_REGISTER_SYMMETRY_INDEX => unimplemented!(),
            ABI_THEOREM_REGISTER_TRANSITIVITY_INDEX => unimplemented!(),
            ABI_THEOREM_REGISTER_BETA_INDEX => unimplemented!(),
            ABI_THEOREM_REGISTER_ETA_INDEX => unimplemented!(),
            ABI_THEOREM_REGISTER_APPLICATION_INDEX => unimplemented!(),
            ABI_THEOREM_REGISTER_LAMBDA_INDEX => unimplemented!(),
            ABI_THEOREM_REGISTER_SUBSTITUTION_INDEX => unimplemented!(),
            ABI_THEOREM_REGISTER_TYPE_SUBSTITUTION_INDEX => unimplemented!(),
            ABI_THEOREM_REGISTER_TRUTH_INTRODUCTION_INDEX => unimplemented!(),
            ABI_THEOREM_REGISTER_FALSITY_ELIMINATION_INDEX => unimplemented!(),
            ABI_THEOREM_REGISTER_CONJUNCTION_INTRODUCTION_INDEX => {
                unimplemented!()
            }
            ABI_THEOREM_REGISTER_CONJUNCTION_LEFT_ELIMINATION_INDEX => {
                unimplemented!()
            }
            ABI_THEOREM_REGISTER_CONJUNCTION_RIGHT_ELIMINATION_INDEX => {
                unimplemented!()
            }
            ABI_THEOREM_REGISTER_DISJUNCTION_ELIMINATION_INDEX => {
                unimplemented!()
            }
            ABI_THEOREM_REGISTER_DISJUNCTION_LEFT_INTRODUCTION_INDEX => {
                unimplemented!()
            }
            ABI_THEOREM_REGISTER_DISJUNCTION_RIGHT_INTRODUCTION_INDEX => {
                unimplemented!()
            }
            ABI_THEOREM_REGISTER_IMPLICATION_INTRODUCTION_INDEX => {
                unimplemented!()
            }
            ABI_THEOREM_REGISTER_IMPLICATION_ELIMINATION_INDEX => {
                unimplemented!()
            }
            ABI_THEOREM_REGISTER_IFF_INTRODUCTION_INDEX => unimplemented!(),
            ABI_THEOREM_REGISTER_IFF_LEFT_ELIMINATION_INDEX => unimplemented!(),
            ABI_THEOREM_REGISTER_NEGATION_INTRODUCTION_INDEX => {
                unimplemented!()
            }
            ABI_THEOREM_REGISTER_NEGATION_ELIMINATION_INDEX => unimplemented!(),
            ABI_THEOREM_REGISTER_FORALL_INTRODUCTION_INDEX => unimplemented!(),
            ABI_THEOREM_REGISTER_FORALL_ELIMINATION_INDEX => unimplemented!(),
            ABI_THEOREM_REGISTER_EXISTS_INTRODUCTION_INDEX => unimplemented!(),
            ABI_THEOREM_REGISTER_EXISTS_ELIMINATION_INDEX => unimplemented!(),
            ABI_THEOREM_REGISTER_WEAKEN_INDEX => unimplemented!(),
            _otherwise => Err(host_trap(RuntimeTrap::NoSuchFunction)),
        }
    }
}

/// Maps an ABI host-call to its associated host-call number.  Also checks that
/// the function's signature is as expected, otherwise produces a runtime error
/// that is reported back to the WASM program.
impl ModuleImportResolver for WasmiRuntimeState {
    fn resolve_func(
        &self,
        field_name: &str,
        signature: &Signature,
    ) -> Result<FuncRef, WasmiError> {
        match field_name {
            ABI_TYPE_FORMER_RESOLVE_NAME => {
                if !check_type_former_resolve_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TYPE_FORMER_RESOLVE_INDEX,
                ))
            }
            ABI_TYPE_FORMER_REGISTER_NAME => {
                if !check_type_former_register_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TYPE_FORMER_REGISTER_INDEX,
                ))
            }
            ABI_TYPE_FORMER_IS_REGISTERED_NAME => {
                if !check_type_former_is_registered_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TYPE_FORMER_IS_REGISTERED_INDEX,
                ))
            }
            ABI_TYPE_IS_REGISTERED_NAME => {
                if !check_type_is_registered_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TYPE_IS_REGISTERED_INDEX,
                ))
            }
            ABI_TYPE_REGISTER_VARIABLE_NAME => {
                if !check_type_register_variable_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TYPE_REGISTER_VARIABLE_INDEX,
                ))
            }
            ABI_TYPE_REGISTER_COMBINATION_NAME => {
                if !check_type_register_combination_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TYPE_REGISTER_COMBINATION_INDEX,
                ))
            }
            ABI_TYPE_REGISTER_FUNCTION_NAME => {
                if !check_type_register_function_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TYPE_REGISTER_FUNCTION_INDEX,
                ))
            }
            ABI_TYPE_SPLIT_VARIABLE_NAME => {
                if !check_type_split_variable_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TYPE_SPLIT_VARIABLE_INDEX,
                ))
            }
            ABI_TYPE_SPLIT_COMBINATION_NAME => {
                if !check_type_split_combination_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TYPE_SPLIT_COMBINATION_INDEX,
                ))
            }
            ABI_TYPE_SPLIT_FUNCTION_NAME => {
                if !check_type_split_function_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TYPE_SPLIT_FUNCTION_INDEX,
                ))
            }
            ABI_TYPE_TEST_VARIABLE_NAME => {
                if !check_type_test_variable_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TYPE_TEST_VARIABLE_INDEX,
                ))
            }
            ABI_TYPE_TEST_COMBINATION_NAME => {
                if !check_type_test_combination_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TYPE_TEST_COMBINATION_INDEX,
                ))
            }
            ABI_TYPE_TEST_FUNCTION_NAME => {
                if !check_type_test_function_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TYPE_TEST_FUNCTION_INDEX,
                ))
            }
            ABI_TYPE_FTV_NAME => {
                if !check_type_ftv_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TYPE_VARIABLES_INDEX,
                ))
            }
            ABI_TYPE_SUBSTITUTE_NAME => {
                if !check_type_substitute_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TYPE_SUBSTITUTE_INDEX,
                ))
            }
            ABI_CONSTANT_RESOLVE_NAME => {
                if !check_constant_resolve_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_CONSTANT_RESOLVE_INDEX,
                ))
            }
            ABI_CONSTANT_IS_REGISTERED_NAME => {
                if !check_constant_is_registered_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_CONSTANT_IS_REGISTERED_INDEX,
                ))
            }
            ABI_CONSTANT_REGISTER_NAME => {
                if !check_constant_register_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_CONSTANT_REGISTER_INDEX,
                ))
            }
            ABI_TERM_REGISTER_VARIABLE_NAME => {
                if !check_term_register_variable_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TERM_REGISTER_VARIABLE_INDEX,
                ))
            }
            ABI_TERM_REGISTER_CONSTANT_NAME => {
                if !check_term_register_constant_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TERM_REGISTER_CONSTANT_INDEX,
                ))
            }
            ABI_TERM_REGISTER_APPLICATION_NAME => {
                if !check_term_register_application_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TERM_REGISTER_APPLICATION_INDEX,
                ))
            }
            ABI_TERM_REGISTER_LAMBDA_NAME => {
                if !check_term_register_lambda_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TERM_REGISTER_LAMBDA_INDEX,
                ))
            }
            ABI_TERM_REGISTER_NEGATION_NAME => {
                if !check_term_register_negation_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TERM_REGISTER_NEGATION_INDEX,
                ))
            }
            ABI_TERM_REGISTER_CONJUNCTION_NAME => {
                if !check_term_register_conjunction_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TERM_REGISTER_CONJUNCTION_INDEX,
                ))
            }
            ABI_TERM_REGISTER_DISJUNCTION_NAME => {
                if !check_term_register_disjunction_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TERM_REGISTER_DISJUNCTION_INDEX,
                ))
            }
            ABI_TERM_REGISTER_IMPLICATION_NAME => {
                if !check_term_register_implication_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TERM_REGISTER_IMPLICATION_INDEX,
                ))
            }
            ABI_TERM_REGISTER_EQUALITY_NAME => {
                if !check_term_register_equality_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TERM_REGISTER_EQUALITY_INDEX,
                ))
            }
            ABI_TERM_REGISTER_FORALL_NAME => {
                if !check_term_register_forall_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TERM_REGISTER_FORALL_INDEX,
                ))
            }
            ABI_TERM_REGISTER_EXISTS_NAME => {
                if !check_term_register_exists_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TERM_REGISTER_EXISTS_INDEX,
                ))
            }
            ABI_TERM_SPLIT_VARIABLE_NAME => {
                if !check_term_split_variable_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TERM_SPLIT_VARIABLE_INDEX,
                ))
            }
            ABI_TERM_SPLIT_CONSTANT_NAME => {
                if !check_term_split_constant_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TERM_SPLIT_CONSTANT_INDEX,
                ))
            }
            ABI_TERM_SPLIT_APPLICATION_NAME => {
                if !check_term_split_application_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TERM_SPLIT_APPLICATION_INDEX,
                ))
            }
            ABI_TERM_SPLIT_LAMBDA_NAME => {
                if !check_term_split_lambda_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TERM_SPLIT_LAMBDA_INDEX,
                ))
            }
            ABI_TERM_SPLIT_NEGATION_NAME => {
                if !check_term_split_negation_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TERM_SPLIT_NEGATION_INDEX,
                ))
            }
            ABI_TERM_SPLIT_CONJUNCTION_NAME => {
                if !check_term_split_conjunction_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TERM_SPLIT_CONJUNCTION_INDEX,
                ))
            }
            ABI_TERM_SPLIT_DISJUNCTION_NAME => {
                if !check_term_split_disjunction_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TERM_SPLIT_DISJUNCTION_INDEX,
                ))
            }
            ABI_TERM_SPLIT_IMPLICATION_NAME => {
                if !check_term_split_implication_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TERM_SPLIT_IMPLICATION_INDEX,
                ))
            }
            ABI_TERM_SPLIT_EQUALITY_NAME => {
                if !check_term_split_equality_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TERM_SPLIT_EQUALITY_INDEX,
                ))
            }
            ABI_TERM_SPLIT_FORALL_NAME => {
                if !check_term_split_forall_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TERM_SPLIT_FORALL_INDEX,
                ))
            }
            ABI_TERM_SPLIT_EXISTS_NAME => {
                if !check_term_split_exists_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TERM_SPLIT_EXISTS_INDEX,
                ))
            }
            ABI_TERM_TEST_VARIABLE_NAME => {
                if !check_term_test_variable_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TERM_TEST_VARIABLE_INDEX,
                ))
            }
            ABI_TERM_TEST_CONSTANT_NAME => {
                if !check_term_test_constant_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TERM_TEST_CONSTANT_INDEX,
                ))
            }
            ABI_TERM_TEST_APPLICATION_NAME => {
                if !check_term_test_application_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TERM_TEST_APPLICATION_INDEX,
                ))
            }
            ABI_TERM_TEST_LAMBDA_NAME => {
                if !check_term_test_lambda_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TERM_TEST_LAMBDA_INDEX,
                ))
            }
            ABI_TERM_TEST_NEGATION_NAME => {
                if !check_term_test_negation_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TERM_TEST_NEGATION_INDEX,
                ))
            }
            ABI_TERM_TEST_CONJUNCTION_NAME => {
                if !check_term_test_conjunction_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TERM_TEST_CONJUNCTION_INDEX,
                ))
            }
            ABI_TERM_TEST_DISJUNCTION_NAME => {
                if !check_term_test_disjunction_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TERM_TEST_DISJUNCTION_INDEX,
                ))
            }
            ABI_TERM_TEST_IMPLICATION_NAME => {
                if !check_term_test_implication_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TERM_TEST_IMPLICATION_INDEX,
                ))
            }
            ABI_TERM_TEST_EQUALITY_NAME => {
                if !check_term_test_equality_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TERM_TEST_EQUALITY_INDEX,
                ))
            }
            ABI_TERM_TEST_FORALL_NAME => {
                if !check_term_test_forall_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TERM_TEST_FORALL_INDEX,
                ))
            }
            ABI_TERM_TEST_EXISTS_NAME => {
                if !check_term_test_exists_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TERM_TEST_EXISTS_INDEX,
                ))
            }
            ABI_TERM_FREE_VARIABLES_NAME => {
                if !check_term_fv_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TERM_FREE_VARIABLES_INDEX,
                ))
            }
            ABI_TERM_SUBSTITUTION_NAME => {
                if !check_term_substitution_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TERM_SUBSTITUTION_INDEX,
                ))
            }
            ABI_TERM_TYPE_VARIABLES_NAME => {
                if !check_term_type_variables_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TERM_TYPE_VARIABLES_INDEX,
                ))
            }
            ABI_TERM_TYPE_SUBSTITUTION_NAME => {
                if !check_term_type_substitution_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TERM_TYPE_SUBSTITUTION_INDEX,
                ))
            }
            ABI_TERM_TYPE_INFER_NAME => {
                if !check_term_type_infer_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TERM_TYPE_INFER_INDEX,
                ))
            }
            ABI_TERM_TYPE_IS_PROPOSITION_NAME => {
                if !check_term_type_is_proposition_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TERM_TYPE_IS_PROPOSITION_INDEX,
                ))
            }
            ABI_THEOREM_IS_REGISTERED_NAME => {
                if !check_theorem_is_registered_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_THEOREM_IS_REGISTERED_INDEX,
                ))
            }
            ABI_THEOREM_REGISTER_ASSUMPTION_NAME => {
                if !check_theorem_register_assumption_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_THEOREM_REGISTER_ASSUMPTION_INDEX,
                ))
            }
            ABI_THEOREM_REGISTER_REFLEXIVITY_NAME => {
                if !check_theorem_register_reflexivity_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_THEOREM_REGISTER_REFLEXIVITY_INDEX,
                ))
            }
            ABI_THEOREM_REGISTER_SYMMETRY_NAME => {
                if !check_theorem_register_symmetry_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_THEOREM_REGISTER_SYMMETRY_INDEX,
                ))
            }
            ABI_THEOREM_REGISTER_TRANSITIVITY_NAME => {
                if !check_theorem_register_transitivity_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_THEOREM_REGISTER_TRANSITIVITY_INDEX,
                ))
            }
            ABI_THEOREM_REGISTER_APPLICATION_NAME => {
                if !check_theorem_register_application_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_THEOREM_REGISTER_APPLICATION_INDEX,
                ))
            }
            ABI_THEOREM_REGISTER_LAMBDA_NAME => {
                if !check_theorem_register_lambda_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_THEOREM_REGISTER_LAMBDA_INDEX,
                ))
            }
            ABI_THEOREM_REGISTER_BETA_NAME => {
                if !check_theorem_register_beta_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_THEOREM_REGISTER_BETA_INDEX,
                ))
            }
            ABI_THEOREM_REGISTER_ETA_NAME => {
                if !check_theorem_register_eta_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_THEOREM_REGISTER_ETA_INDEX,
                ))
            }
            ABI_THEOREM_REGISTER_TRUTH_INTRODUCTION_NAME => {
                if !check_theorem_register_truth_introduction_signature(
                    signature,
                ) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_THEOREM_REGISTER_TRUTH_INTRODUCTION_INDEX,
                ))
            }
            ABI_THEOREM_REGISTER_FALSITY_ELIMINATION_NAME => {
                if !check_theorem_register_falsity_elimination_signature(
                    signature,
                ) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_THEOREM_REGISTER_FALSITY_ELIMINATION_INDEX,
                ))
            }
            ABI_THEOREM_REGISTER_CONJUNCTION_INTRODUCTION_NAME => {
                if !check_theorem_register_conjunction_introduction_signature(
                    signature,
                ) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_THEOREM_REGISTER_CONJUNCTION_INTRODUCTION_INDEX,
                ))
            }
            ABI_THEOREM_REGISTER_CONJUNCTION_LEFT_ELIMINATION_NAME => {
                if !check_theorem_register_conjunction_left_elimination_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_THEOREM_REGISTER_CONJUNCTION_LEFT_ELIMINATION_INDEX,
                ))
            }
            ABI_THEOREM_REGISTER_CONJUNCTION_RIGHT_ELIMINATION_NAME => {
                if !check_theorem_register_conjunction_right_elimination_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_THEOREM_REGISTER_CONJUNCTION_RIGHT_ELIMINATION_INDEX,
                ))
            }
            ABI_THEOREM_REGISTER_DISJUNCTION_ELIMINATION_NAME => {
                if !check_theorem_register_disjunction_elimination_signature(
                    signature,
                ) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_THEOREM_REGISTER_DISJUNCTION_ELIMINATION_INDEX,
                ))
            }
            ABI_THEOREM_REGISTER_DISJUNCTION_LEFT_INTRODUCTION_NAME => {
                if !check_theorem_register_disjunction_left_introduction_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_THEOREM_REGISTER_DISJUNCTION_LEFT_INTRODUCTION_INDEX,
                ))
            }
            ABI_THEOREM_REGISTER_DISJUNCTION_RIGHT_INTRODUCTION_NAME => {
                if !check_theorem_register_disjunction_right_introduction_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_THEOREM_REGISTER_DISJUNCTION_RIGHT_INTRODUCTION_INDEX,
                ))
            }
            ABI_THEOREM_REGISTER_IMPLICATION_INTRODUCTION_NAME => {
                if !check_theorem_register_implication_introduction_signature(
                    signature,
                ) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_THEOREM_REGISTER_IMPLICATION_INTRODUCTION_INDEX,
                ))
            }
            ABI_THEOREM_REGISTER_IMPLICATION_ELIMINATION_NAME => {
                if !check_theorem_register_implication_elimination_signature(
                    signature,
                ) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_THEOREM_REGISTER_IMPLICATION_ELIMINATION_INDEX,
                ))
            }
            ABI_THEOREM_REGISTER_IFF_INTRODUCTION_NAME => {
                if !check_theorem_register_iff_introduction_signature(signature)
                {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_THEOREM_REGISTER_IFF_INTRODUCTION_INDEX,
                ))
            }
            ABI_THEOREM_REGISTER_IFF_LEFT_ELIMINATION_NAME => {
                if !check_theorem_register_iff_left_elimination_signature(
                    signature,
                ) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_THEOREM_REGISTER_IFF_LEFT_ELIMINATION_INDEX,
                ))
            }
            ABI_THEOREM_REGISTER_NEGATION_INTRODUCTION_NAME => {
                if !check_theorem_register_negation_introduction_signature(
                    signature,
                ) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_THEOREM_REGISTER_NEGATION_INTRODUCTION_INDEX,
                ))
            }
            ABI_THEOREM_REGISTER_NEGATION_ELIMINATION_NAME => {
                if !check_theorem_register_negation_elimination_signature(
                    signature,
                ) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_THEOREM_REGISTER_NEGATION_ELIMINATION_INDEX,
                ))
            }
            ABI_THEOREM_REGISTER_FORALL_INTRODUCTION_NAME => {
                if !check_theorem_register_forall_introduction_signature(
                    signature,
                ) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_THEOREM_REGISTER_FORALL_INTRODUCTION_INDEX,
                ))
            }
            ABI_THEOREM_REGISTER_FORALL_ELIMINATION_NAME => {
                if !check_theorem_register_forall_elimination_signature(
                    signature,
                ) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_THEOREM_REGISTER_FORALL_ELIMINATION_INDEX,
                ))
            }
            ABI_THEOREM_REGISTER_EXISTS_ELIMINATION_NAME => {
                if !check_theorem_register_exists_elimination_signature(
                    signature,
                ) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_THEOREM_REGISTER_EXISTS_ELIMINATION_INDEX,
                ))
            }
            ABI_THEOREM_REGISTER_EXISTS_INTRODUCTION_NAME => {
                if !check_theorem_register_exists_introduction_signature(
                    signature,
                ) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_THEOREM_REGISTER_EXISTS_INTRODUCTION_INDEX,
                ))
            }
            ABI_THEOREM_REGISTER_WEAKEN_NAME => {
                if !check_theorem_register_weaken_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_THEOREM_REGISTER_WEAKEN_INDEX,
                ))
            }
            ABI_THEOREM_SPLIT_CONCLUSION_NAME => {
                if !check_theorem_split_conclusion_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_THEOREM_SPLIT_CONCLUSION_INDEX,
                ))
            }
            ABI_THEOREM_SPLIT_HYPOTHESES_NAME => {
                if !check_theorem_split_hypotheses_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_THEOREM_SPLIT_HYPOTHESES_INDEX,
                ))
            }
            _otherwise => Err(host_error(KernelErrorCode::NoSuchFunction)),
        }
    }
}
