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
    Error as WasmiError, Externals, FuncInstance, FuncRef, HostError, MemoryInstance,
    ModuleImportResolver, RuntimeArgs, RuntimeValue, Signature, Trap, TrapKind, ValueType,
};

use crate::kernel::{
    error_code::ErrorCode as KernelErrorCode,
    handle::{tags, Handle},
    name::Name,
    runtime_state::RuntimeState as KernelRuntimeState,
};

////////////////////////////////////////////////////////////////////////////////
// Miscellaneous material.
////////////////////////////////////////////////////////////////////////////////

/// A WASM address, used for reading-from and writing-to the guest WASM program
/// heap, assuming the `wasm32-abi`.
pub type Address = u32;

////////////////////////////////////////////////////////////////////////////////
// ABI: host-call names and numbers.
////////////////////////////////////////////////////////////////////////////////

/* Type-former related calls. */

/// The name of the `TypeFormer.Resolve` ABI call.
pub const ABI_TYPE_FORMER_RESOLVE_NAME: &'static str = "__type_former_resolve";
/// The name of the `TypeFormer.IsRegistered` ABI call.
pub const ABI_TYPE_FORMER_IS_REGISTERED_NAME: &'static str = "__type_former_is_registered";
/// The name of the `TypeFormer.Register` ABI call.
pub const ABI_TYPE_FORMER_REGISTER_NAME: &'static str = "__type_former_register";

/// The host-call number of the `TypeFormer.Resolve` ABI call.
pub const ABI_TYPE_FORMER_RESOLVE_INDEX: usize = 0;
/// The host-call number of the `TypeFormer.IsRegistered` ABI call.
pub const ABI_TYPE_FORMER_IS_REGISTERED_INDEX: usize = 1;
/// The host-call number of the `TypeFormer.Register` ABI call.
pub const ABI_TYPE_FORMER_REGISTER_INDEX: usize = 2;

/* Type-related calls. */

/// The name of the `Type.IsRegistered` ABI call.
pub const ABI_TYPE_IS_REGISTERED_NAME: &'static str = "__type_is_registered";
/// The name of the `Type.Register.Variable` ABI call.
pub const ABI_TYPE_REGISTER_VARIABLE_NAME: &'static str = "__type_register_variable";
/// The name of the `Type.Register.Combination` ABI call.
pub const ABI_TYPE_REGISTER_COMBINATION_NAME: &'static str = "__type_register_combination_name";
/// The name of the `Type.Register.Function` ABI call.
pub const ABI_TYPE_REGISTER_FUNCTION_NAME: &'static str = "__type_register_function_name";

/// The name of the `Type.Split.Variable` ABI call.
pub const ABI_TYPE_SPLIT_VARIABLE_NAME: &'static str = "__type_split_variable_name";
/// The name of the `Type.Split.Combination` ABI call.
pub const ABI_TYPE_SPLIT_COMBINATION_NAME: &'static str = "__type_split_combination_name";
/// The name of the `Type.Split.Function` ABI call.
pub const ABI_TYPE_SPLIT_FUNCTION_NAME: &'static str = "__type_split_function_name";

/// The name of the `Type.Test.Variable` ABI call.
pub const ABI_TYPE_TEST_VARIABLE_NAME: &'static str = "__type_test_variable";
/// The name of the `Type.Test.Combination` ABI call.
pub const ABI_TYPE_TEST_COMBINATION_NAME: &'static str = "__type_test_combination";
/// The name of the `Type.Test.Function` ABI call.
pub const ABI_TYPE_TEST_FUNCTION_NAME: &'static str = "__type_test_function";

/// The name of the `Type.Variables` ABI call.
pub const ABI_TYPE_FTV_NAME: &'static str = "__type_variables";
/// The name of the `Type.Substitute` ABI call.
pub const ABI_TYPE_SUBSTITUTE_NAME: &'static str = "__type_substitute";

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
pub const ABI_CONSTANT_RESOLVE_NAME: &'static str = "__constant_resolve";
/// The name of the `Constant.IsRegistered` ABI call.
pub const ABI_CONSTANT_IS_REGISTERED_NAME: &'static str = "__constant_is_registered";
/// The name of the `Constant.Register` ABI call.
pub const ABI_CONSTANT_REGISTER_NAME: &'static str = "__constant_register";

/// The host-call number of the `Constant.Resolve` ABI call.
pub const ABI_CONSTANT_RESOLVE_INDEX: usize = 15;
/// The host-call number of the `Constant.IsRegistered` ABI call.
pub const ABI_CONSTANT_IS_REGISTERED_INDEX: usize = 16;
/// The host-call number of the `Constant.Register` ABI call.
pub const ABI_CONSTANT_REGISTER_INDEX: usize = 17;

/* Term-related calls. */

/// The name of the `Term.Register.Variable` ABI call.
pub const ABI_TERM_REGISTER_VARIABLE_NAME: &'static str = "__term_register_variable";
/// The name of the `Term.Register.Constant` ABI call.
pub const ABI_TERM_REGISTER_CONSTANT_NAME: &'static str = "__term_register_constant";
/// The name of the `Term.Register.Application` ABI call.
pub const ABI_TERM_REGISTER_APPLICATION_NAME: &'static str = "__term_register_application";
/// The name of the `Term.Register.Lambda` ABI call.
pub const ABI_TERM_REGISTER_LAMBDA_NAME: &'static str = "__term_register_lambda";
/// The name of the `Term.Register.Negation` ABI call.
pub const ABI_TERM_REGISTER_NEGATION_NAME: &'static str = "__term_register_negation";
/// The name of the `Term.Register.Conjunction` ABI call.
pub const ABI_TERM_REGISTER_CONJUNCTION_NAME: &'static str = "__term_register_conjunction";
/// The name of the `Term.Register.Disjunction` ABI call.
pub const ABI_TERM_REGISTER_DISJUNCTION_NAME: &'static str = "__term_register_disjunction";
/// The name of the `Term.Register.Implication` ABI call.
pub const ABI_TERM_REGISTER_IMPLICATION_NAME: &'static str = "__term_register_implication";
/// The name of the `Term.Register.Equality` ABI call.
pub const ABI_TERM_REGISTER_EQUALITY_NAME: &'static str = "__term_register_equality";
/// The name of the `Term.Register.Forall` ABI call.
pub const ABI_TERM_REGISTER_FORALL_NAME: &'static str = "__term_register_forall";
/// The name of the `Term.Register.Exists` ABI call.
pub const ABI_TERM_REGISTER_EXISTS_NAME: &'static str = "__term_register_exists";

/// The name of the `Term.Split.Variable` ABI call.
pub const ABI_TERM_SPLIT_VARIABLE_NAME: &'static str = "__term_split_variable";
/// The name of the `Term.Split.Constant` ABI call.
pub const ABI_TERM_SPLIT_CONSTANT_NAME: &'static str = "__term_split_constant";
/// The name of the `Term.Split.Application` ABI call.
pub const ABI_TERM_SPLIT_APPLICATION_NAME: &'static str = "__term_split_application";
/// The name of the `Term.Split.Lambda` ABI call.
pub const ABI_TERM_SPLIT_LAMBDA_NAME: &'static str = "__term_split_lambda";
/// The name of the `Term.Split.Negation` ABI call.
pub const ABI_TERM_SPLIT_NEGATION_NAME: &'static str = "__term_split_negation";
/// The name of the `Term.Split.Conjunction` ABI call.
pub const ABI_TERM_SPLIT_CONJUNCTION_NAME: &'static str = "__term_split_conjunction";
/// The name of the `Term.Split.Disjunction` ABI call.
pub const ABI_TERM_SPLIT_DISJUNCTION_NAME: &'static str = "__term_split_disjunction";
/// The name of the `Term.Split.Implication` ABI call.
pub const ABI_TERM_SPLIT_IMPLICATION_NAME: &'static str = "__term_split_implication";
/// The name of the `Term.Split.Equality` ABI call.
pub const ABI_TERM_SPLIT_EQUALITY_NAME: &'static str = "__term_split_equality";
/// The name of the `Term.Split.Forall` ABI call.
pub const ABI_TERM_SPLIT_FORALL_NAME: &'static str = "__term_split_forall";
/// The name of the `Term.Split.Exists` ABI call.
pub const ABI_TERM_SPLIT_EXISTS_NAME: &'static str = "__term_split_exists";

/// The name of the `Term.Test.Variable` ABI call.
pub const ABI_TERM_TEST_VARIABLE_NAME: &'static str = "__term_test_variable";
/// The name of the `Term.Test.Constant` ABI call.
pub const ABI_TERM_TEST_CONSTANT_NAME: &'static str = "__term_test_constant";
/// The name of the `Term.Test.Application` ABI call.
pub const ABI_TERM_TEST_APPLICATION_NAME: &'static str = "__term_test_application";
/// The name of the `Term.Test.Lambda` ABI call.
pub const ABI_TERM_TEST_LAMBDA_NAME: &'static str = "__term_test_lambda";
/// The name of the `Term.Test.Negation` ABI call.
pub const ABI_TERM_TEST_NEGATION_NAME: &'static str = "__term_test_negation";
/// The name of the `Term.Test.Conjunction` ABI call.
pub const ABI_TERM_TEST_CONJUNCTION_NAME: &'static str = "__term_test_conjunction";
/// The name of the `Term.Test.Disjunction` ABI call.
pub const ABI_TERM_TEST_DISJUNCTION_NAME: &'static str = "__term_test_disjunction";
/// The name of the `Term.Test.Implication` ABI call.
pub const ABI_TERM_TEST_IMPLICATION_NAME: &'static str = "__term_test_implication";
/// The name of the `Term.Test.Equality` ABI call.
pub const ABI_TERM_TEST_EQUALITY_NAME: &'static str = "__term_test_equality";
/// The name of the `Term.Test.Forall` ABI call.
pub const ABI_TERM_TEST_FORALL_NAME: &'static str = "__term_test_forall";
/// The name of the `Term.Test.Exists` ABI call.
pub const ABI_TERM_TEST_EXISTS_NAME: &'static str = "__term_test_exists";

/// The name of the `Term.FreeVariables` ABI call.
pub const ABI_TERM_FV_NAME: &'static str = "__term_fv";
/// The name of the `Term.Substitution` ABI call.
pub const ABI_TERM_SUBSTITUTION_NAME: &'static str = "__term_substitution";

/// The name of the `Term.Type.Variables` ABI call.
pub const ABI_TERM_TYPE_VARIABLES_NAME: &'static str = "__term_type_variables";
/// The name of the `Term.Type.Substitution` ABI call.
pub const ABI_TERM_TYPE_SUBSTITUTION_NAME: &'static str = "__term_type_substitution";
/// The name of the `Term.Type.Infer` ABI call.
pub const ABI_TERM_TYPE_INFER_NAME: &'static str = "__term_type_infer";
/// The name of the `Term.Type.IsProposition` ABI call.
pub const ABI_TERM_TYPE_IS_PROPOSITION_NAME: &'static str = "__term_type_is_proposition";

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
pub const ABI_TERM_FV_INDEX: usize = 51;
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
pub const ABI_THEOREM_IS_REGISTERED_NAME: &'static str = "__theorem_is_registered";

/// The name of the `Theorem.Register.Reflexivity` ABI call.
pub const ABI_THEOREM_REGISTER_REFLEXIVITY_NAME: &'static str = "__theorem_register_reflexivity";
/// The name of the `Theorem.Register.Symmetry` ABI call.
pub const ABI_THEOREM_REGISTER_SYMMETRY_NAME: &'static str = "__theorem_register_symmetry";
/// The name of the `Theorem.Register.Transitivity` ABI call.
pub const ABI_THEOREM_REGISTER_TRANSITIVITY_NAME: &'static str = "__theorem_register_transitivity";
/// The name of the `Theorem.Register.Beta` ABI call.
pub const ABI_THEOREM_REGISTER_BETA_NAME: &'static str = "__theorem_register_beta";
/// The name of the `Theorem.Register.Eta` ABI call.
pub const ABI_THEOREM_REGISTER_ETA_NAME: &'static str = "__theorem_register_eta";
/// The name of the `Theorem.Register.Application` ABI call.
pub const ABI_THEOREM_REGISTER_APPLICATION_NAME: &'static str = "__theorem_register_application";
/// The name of the `Theorem.Register.Lambda` ABI call.
pub const ABI_THEOREM_REGISTER_LAMBDA_NAME: &'static str = "__theorem_register_lambda";

/// The name of the `Theorem.Register.Substitution` ABI call.
pub const ABI_THEOREM_REGISTER_SUBSTITUTION_NAME: &'static str = "__theorem_register_substitution";
/// The name of the `Theorem.Register.TypeSubstitution` ABI call.
pub const ABI_THEOREM_REGISTER_TYPE_SUBSTITUTION_NAME: &'static str =
    "__theorem_register_type_substitution";

/// The name of the `Theorem.Register.TruthIntroduction` ABI call.
pub const ABI_THEOREM_REGISTER_TRUTH_INTRODUCTION_NAME: &'static str =
    "__theorem_register_truth_introduction";
/// The name of the `Theorem.Register.FalsityElimination` ABI call.
pub const ABI_THEOREM_REGISTER_FALSITY_ELIMINATION_NAME: &'static str =
    "__theorem_register_falsity_elimination";

/// The name of the `Theorem.Register.ConjunctionIntroduction` ABI call.
pub const ABI_THEOREM_REGISTER_CONJUNCTION_INTRODUCTION_NAME: &'static str =
    "__theorem_register_conjunction_introduction";
/// The name of the `Theorem.Register.ConjunctionLeftElimination` ABI call.
pub const ABI_THEOREM_REGISTER_CONJUNCTION_LEFT_ELIMINATION_NAME: &'static str =
    "__theorem_register_conjunction_left_elimination";
/// The name of the `Theorem.Register.ConjunctionRightElimination` ABI call.
pub const ABI_THEOREM_REGISTER_CONJUNCTION_RIGHT_ELIMINATION_NAME: &'static str =
    "__theorem_register_conjunction_right_elimination";

/// The name of the `Theorem.Register.DisjunctionElimination` ABI call.
pub const ABI_THEOREM_REGISTER_DISJUNCTION_ELIMINATION_NAME: &'static str =
    "__theorem_register_disjunction_elimination";
/// The name of the `Theorem.Register.DisjunctionLeftIntroduction` ABI call.
pub const ABI_THEOREM_REGISTER_DISJUNCTION_LEFT_INTRODUCTION_NAME: &'static str =
    "__theorem_register_disjunction_left_introduction";
/// The name of the `Theorem.Register.DisjunctionRightIntroduction` ABI call.
pub const ABI_THEOREM_REGISTER_DISJUNCTION_RIGHT_INTRODUCTION_NAME: &'static str =
    "__theorem_register_disjunction_right_introduction";

/// The name of the `Theorem.Register.ImplicationIntroduction` ABI call.
pub const ABI_THEOREM_REGISTER_IMPLICATION_INTRODUCTION_NAME: &'static str =
    "__theorem_register_implication_introduction";
/// The name of the `Theorem.Register.ImplicationElimination` ABI call.
pub const ABI_THEOREM_REGISTER_IMPLICATION_ELIMINATION_NAME: &'static str =
    "__theorem_register_implication_elimination";

/// The name of the `Theorem.Register.IffIntroduction` ABI call.
pub const ABI_THEOREM_REGISTER_IFF_INTRODUCTION_NAME: &'static str =
    "__theorem_register_iff_elimination";
/// The name of the `Theorem.Register.IffLeftElimination` ABI call.
pub const ABI_THEOREM_REGISTER_IFF_LEFT_ELIMINATION_NAME: &'static str =
    "__theorem_register_iff_left_elimination";

/// The name of the `Theorem.Register.NegationIntroduction` ABI call.
pub const ABI_THEOREM_REGISTER_NEGATION_INTRODUCTION_NAME: &'static str =
    "__theorem_register_negation_introduction";
/// The name of the `Theorem.Register.NegationElimination` ABI call.
pub const ABI_THEOREM_REGISTER_NEGATION_ELIMINATION_NAME: &'static str =
    "__theorem_register_negation_elimination";

/// The name of the `Theorem.Register.ForallIntroduction` ABI call.
pub const ABI_THEOREM_REGISTER_FORALL_INTRODUCTION_NAME: &'static str =
    "__theorem_register_forall_introduction";
/// The name of the `Theorem.Register.ForallElimination` ABI call.
pub const ABI_THEOREM_REGISTER_FORALL_ELIMINATION_NAME: &'static str =
    "__theorem_register_forall_elimination";
/// The name of the `Theorem.Register.ExistsIntroduction` ABI call.
pub const ABI_THEOREM_REGISTER_EXISTS_INTRODUCTION_NAME: &'static str =
    "__theorem_register_exists_introduction";
/// The name of the `Theorem.Register.ExistsElimination` ABI call.
pub const ABI_THEOREM_REGISTER_EXISTS_ELIMINATION_NAME: &'static str =
    "__theorem_register_exists_elimination";

/// The name of the `Theorem.Split.Hypotheses` ABI call.
pub const ABI_THEOREM_SPLIT_HYPOTHESES_NAME: &'static str = "__theorem_split_hypotheses";
/// The name of the `Theorem.Split.Conclusion` ABI call.
pub const ABI_THEOREM_SPLIT_CONCLUSION_NAME: &'static str = "__theorem_split_conclusion";

/// The index of the `Theorem.IsRegistered` ABI call.
pub const ABI_THEOREM_IS_REGISTERED_INDEX: usize = 57;

/// The index of the `Theorem.Register.Reflexivity` ABI call.
pub const ABI_THEOREM_REGISTER_REFLEXIVITY_INDEX: usize = 58;
/// The index of the `Theorem.Register.Symmetry` ABI call.
pub const ABI_THEOREM_REGISTER_SYMMETRY_INDEX: usize = 59;
/// The index of the `Theorem.Register.Transitivity` ABI call.
pub const ABI_THEOREM_REGISTER_TRANSITIVITY_INDEX: usize = 60;
/// The index of the `Theorem.Register.Beta` ABI call.
pub const ABI_THEOREM_REGISTER_BETA_INDEX: usize = 61;
/// The index of the `Theorem.Register.Eta` ABI call.
pub const ABI_THEOREM_REGISTER_ETA_INDEX: usize = 62;
/// The index of the `Theorem.Register.Application` ABI call.
pub const ABI_THEOREM_REGISTER_APPLICATION_INDEX: usize = 63;
/// The index of the `Theorem.Register.Lambda` ABI call.
pub const ABI_THEOREM_REGISTER_LAMBDA_INDEX: usize = 64;

/// The index of the `Theorem.Register.Substitution` ABI call.
pub const ABI_THEOREM_REGISTER_SUBSTITUTION_INDEX: usize = 65;
/// The index of the `Theorem.Register.TypeSubstitution` ABI call.
pub const ABI_THEOREM_REGISTER_TYPE_SUBSTITUTION_INDEX: usize = 66;

/// The index of the `Theorem.Register.TruthIntroduction` ABI call.
pub const ABI_THEOREM_REGISTER_TRUTH_INTRODUCTION_INDEX: usize = 67;
/// The index of the `Theorem.Register.FalsityElimination` ABI call.
pub const ABI_THEOREM_REGISTER_FALSITY_ELIMINATION_INDEX: usize = 68;

/// The index of the `Theorem.Register.ConjunctionIntroduction` ABI call.
pub const ABI_THEOREM_REGISTER_CONJUNCTION_INTRODUCTION_INDEX: usize = 69;
/// The index of the `Theorem.Register.ConjunctionLeftElimination` ABI call.
pub const ABI_THEOREM_REGISTER_CONJUNCTION_LEFT_ELIMINATION_INDEX: usize = 70;
/// The index of the `Theorem.Register.ConjunctionRightElimination` ABI call.
pub const ABI_THEOREM_REGISTER_CONJUNCTION_RIGHT_ELIMINATION_INDEX: usize = 71;

/// The index of the `Theorem.Register.DisjunctionElimination` ABI call.
pub const ABI_THEOREM_REGISTER_DISJUNCTION_ELIMINATION_INDEX: usize = 72;
/// The index of the `Theorem.Register.DisjunctionLeftIntroduction` ABI call.
pub const ABI_THEOREM_REGISTER_DISJUNCTION_LEFT_INTRODUCTION_INDEX: usize = 73;
/// The index of the `Theorem.Register.DisjunctionRightIntroduction` ABI call.
pub const ABI_THEOREM_REGISTER_DISJUNCTION_RIGHT_INTRODUCTION_INDEX: usize = 74;

/// The index of the `Theorem.Register.ImplicationIntroduction` ABI call.
pub const ABI_THEOREM_REGISTER_IMPLICATION_INTRODUCTION_INDEX: usize = 75;
/// The index of the `Theorem.Register.ImplicationElimination` ABI call.
pub const ABI_THEOREM_REGISTER_IMPLICATION_ELIMINATION_INDEX: usize = 76;

/// The index of the `Theorem.Register.IffIntroduction` ABI call.
pub const ABI_THEOREM_REGISTER_IFF_INTRODUCTION_INDEX: usize = 77;
/// The index of the `Theorem.Register.IffLeftElimination` ABI call.
pub const ABI_THEOREM_REGISTER_IFF_LEFT_ELIMINATION_INDEX: usize = 78;

/// The index of the `Theorem.Register.NegationIntroduction` ABI call.
pub const ABI_THEOREM_REGISTER_NEGATION_INTRODUCTION_INDEX: usize = 79;
/// The index of the `Theorem.Register.NegationElimination` ABI call.
pub const ABI_THEOREM_REGISTER_NEGATION_ELIMINATION_INDEX: usize = 80;

/// The index of the `Theorem.Register.ForallIntroduction` ABI call.
pub const ABI_THEOREM_REGISTER_FORALL_INTRODUCTION_INDEX: usize = 81;
/// The index of the `Theorem.Register.ForallElimination` ABI call.
pub const ABI_THEOREM_REGISTER_FORALL_ELIMINATION_INDEX: usize = 82;
/// The index of the `Theorem.Register.ExistsIntroduction` ABI call.
pub const ABI_THEOREM_REGISTER_EXISTS_INTRODUCTION_INDEX: usize = 83;
/// The index of the `Theorem.Register.ExistsElimination` ABI call.
pub const ABI_THEOREM_REGISTER_EXISTS_ELIMINATION_INDEX: usize = 84;

/// The index of the `Theorem.Split.Hypotheses` ABI call.
pub const ABI_THEOREM_SPLIT_HYPOTHESES_INDEX: usize = 85;
/// The index of the `Theorem.Split.Conclusion` ABI call.
pub const ABI_THEOREM_SPLIT_CONCLUSION_INDEX: usize = 86;

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

impl Display for RuntimeTrap {
    fn fmt(&self, f: &mut Formatter) -> Result<(), DisplayError> {
        match self {
            RuntimeTrap::NoSuchFunction => write!(f, "NoSuchFunction"),
            RuntimeTrap::SignatureFailure => write!(f, "SignatureFailure"),
            RuntimeTrap::MemoryNotRegistered => write!(f, "MemoryNotRegistered"),
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
    fn write_bytes<T>(&self, address: T, bytes: &[u8]) -> Result<(), RuntimeTrap>
    where
        T: Into<Address>,
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

    /// Writes an `i32` value to the WASM guest's memory module at a specified
    /// address.
    ///
    /// # Errors
    ///
    /// Returns `Err(RuntimeTrap::MemoryNotRegistered)` if the WASM guest's
    /// memory module has not been registered with the runtime state.
    ///
    /// Returns `Err(RuntimeTrap::MemoryWriteFailed)` if the write to memory at
    /// address, `address`, failed.
    pub fn write_i32<T, U>(&self, address: T, value: U) -> Result<(), RuntimeTrap>
    where
        T: Into<Address>,
        U: Into<i32>,
    {
        let mut buffer = Vec::new();
        LittleEndian::write_i32(&mut buffer, value.into());

        self.write_bytes(address, &buffer)
    }

    /// Writes a `i64` value to the WASM guest's memory module at a specified
    /// address.
    ///
    /// # Errors
    ///
    /// Returns `Err(RuntimeTrap::MemoryNotRegistered)` if the WASM guest's
    /// memory module has not been registered with the runtime state.
    ///
    /// Returns `Err(RuntimeTrap::MemoryWriteFailed)` if the write to memory at
    /// address, `address`, failed.
    pub fn write_i64<T, U>(&self, address: T, value: U) -> Result<(), RuntimeTrap>
    where
        T: Into<Address>,
        U: Into<i64>,
    {
        let mut buffer = Vec::new();
        LittleEndian::write_i64(&mut buffer, value.into());

        self.write_bytes(address, &buffer)
    }

    /// Writes a `u32` value to the WASM guest's memory module at a specified
    /// address.
    ///
    /// # Errors
    ///
    /// Returns `Err(RuntimeTrap::MemoryNotRegistered)` if the WASM guest's
    /// memory module has not been registered with the runtime state.
    ///
    /// Returns `Err(RuntimeTrap::MemoryWriteFailed)` if the write to memory at
    /// address, `address`, failed.
    pub fn write_u32<T, U>(&self, address: T, value: U) -> Result<(), RuntimeTrap>
    where
        T: Into<Address>,
        U: Into<u32>,
    {
        let mut buffer = Vec::new();
        LittleEndian::write_u32(&mut buffer, value.into());

        self.write_bytes(address, &buffer)
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
    pub fn write_u64<T, U>(&self, address: T, value: U) -> Result<(), RuntimeTrap>
    where
        T: Into<Address>,
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
    pub fn write_u64s<T, U>(&self, address: T, values: Vec<U>) -> Result<(), RuntimeTrap>
    where
        T: Into<Address>,
        U: Into<u64> + Clone,
    {
        let mut address = address.into();

        for v in values.iter().cloned() {
            self.write_u64(address, v)?;
            address += 8;
        }

        Ok(())
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
    pub fn write_handle<T, U, V>(&self, address: T, handle: U) -> Result<(), RuntimeTrap>
    where
        T: Into<Address>,
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
    pub fn write_handles<T, U, V>(&self, address: T, handles: Vec<U>) -> Result<(), RuntimeTrap>
    where
        T: Into<Address>,
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
    fn read_bytes<T, U>(&self, address: T, byte_count: U) -> Result<Vec<u8>, RuntimeTrap>
    where
        T: Into<Address>,
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

    #[inline]
    pub fn read_u32<T>(&self, address: T) -> Result<u32, RuntimeTrap>
    where
        T: Into<Address>,
    {
        let buffer = self.read_bytes(address, size_of::<u32>())?;
        Ok(LittleEndian::read_u32(&buffer))
    }

    #[inline]
    pub fn read_u64<T>(&self, address: T) -> Result<u64, RuntimeTrap>
    where
        T: Into<Address>,
    {
        let buffer = self.read_bytes(address, size_of::<u64>())?;
        Ok(LittleEndian::read_u64(&buffer))
    }

    #[inline]
    pub fn read_i32<T>(&self, address: T) -> Result<i32, RuntimeTrap>
    where
        T: Into<Address>,
    {
        let buffer = self.read_bytes(address, size_of::<i32>())?;
        Ok(LittleEndian::read_i32(&buffer))
    }

    #[inline]
    pub fn read_i64<T>(&self, address: T) -> Result<i64, RuntimeTrap>
    where
        T: Into<Address>,
    {
        let buffer = self.read_bytes(address, size_of::<i32>())?;
        Ok(LittleEndian::read_i64(&buffer))
    }

    #[inline]
    pub fn read_handle<T, U>(&self, address: U) -> Result<Handle<T>, RuntimeTrap>
    where
        T: tags::IsTag,
        U: Into<u32>,
    {
        Ok(Handle::from(self.read_u64(address)? as usize))
    }

    pub fn read_handles<T, U, V>(&self, address: U, count: V) -> Result<Vec<Handle<T>>, RuntimeTrap>
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

    #[inline]
    pub fn type_former_resolve<T>(&self, handle: T) -> Option<usize>
    where
        T: Borrow<Handle<tags::TypeFormer>>,
    {
        self.kernel.borrow().type_former_resolve(handle).cloned()
    }

    #[inline]
    pub fn type_former_is_registered<T>(&self, handle: T) -> bool
    where
        T: Borrow<Handle<tags::TypeFormer>>,
    {
        self.kernel.borrow().type_former_is_registered(handle)
    }

    #[inline]
    pub fn type_former_register<T>(&self, arity: T) -> Handle<tags::TypeFormer>
    where
        T: Into<usize> + Clone,
    {
        self.kernel.borrow_mut().type_former_register(arity)
    }

    #[inline]
    pub fn type_register_variable<T>(&self, name: T) -> Handle<tags::Type>
    where
        T: Into<Name> + Clone,
    {
        self.kernel.borrow_mut().type_register_variable(name)
    }

    #[inline]
    pub fn type_register_combination<T, U>(
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

    #[inline]
    pub fn type_register_function<T>(
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

    #[inline]
    pub fn type_is_registered<T>(&self, handle: T) -> bool
    where
        T: Borrow<Handle<tags::Type>>,
    {
        self.kernel.borrow().type_is_registered(handle)
    }

    #[inline]
    pub fn type_split_variable<T>(&self, handle: T) -> Result<Name, KernelErrorCode>
    where
        T: Borrow<Handle<tags::Type>>,
    {
        self.kernel
            .borrow()
            .type_split_variable(handle)
            .map(|n| n.clone())
    }

    #[inline]
    pub fn type_split_combination<T>(
        &self,
        handle: T,
    ) -> Result<(Handle<tags::TypeFormer>, Vec<Handle<tags::Type>>), KernelErrorCode>
    where
        T: Borrow<Handle<tags::Type>>,
    {
        self.kernel
            .borrow()
            .type_split_combination(handle)
            .map(|(f, a)| (f.clone(), a.clone()))
    }

    #[inline]
    pub fn type_split_function<T>(
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

    #[inline]
    pub fn type_test_variable<T>(&self, handle: T) -> Result<bool, KernelErrorCode>
    where
        T: Borrow<Handle<tags::Type>>,
    {
        self.kernel.borrow().type_test_variable(handle)
    }

    #[inline]
    pub fn type_test_combination<T>(&self, handle: T) -> Result<bool, KernelErrorCode>
    where
        T: Borrow<Handle<tags::Type>>,
    {
        self.kernel.borrow().type_test_combination(handle)
    }

    #[inline]
    pub fn type_test_function<T>(&self, handle: T) -> Result<bool, KernelErrorCode>
    where
        T: Borrow<Handle<tags::Type>>,
    {
        self.kernel.borrow().type_test_function(handle)
    }

    #[inline]
    pub fn type_variables<T>(&self, handle: T) -> Result<Vec<Name>, KernelErrorCode>
    where
        T: Borrow<Handle<tags::Type>>,
    {
        self.kernel
            .borrow_mut()
            .type_variables(handle)
            .map(|v| v.iter().map(|e| *e.clone()).collect())
    }

    #[inline]
    pub fn type_substitute<T, U, V>(
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

    #[inline]
    pub fn constant_register<T>(&self, handle: T) -> Result<Handle<tags::Constant>, KernelErrorCode>
    where
        T: Into<Handle<tags::Type>> + Clone,
    {
        self.kernel.borrow_mut().constant_register(handle)
    }

    #[inline]
    pub fn constant_resolve<T>(&self, handle: T) -> Result<Handle<tags::Type>, KernelErrorCode>
    where
        T: Borrow<Handle<tags::Constant>>,
    {
        self.kernel
            .borrow()
            .constant_resolve(handle)
            .map(|e| e.clone())
    }

    #[inline]
    pub fn constant_is_registered<T>(&self, handle: T) -> bool
    where
        T: Borrow<Handle<tags::Constant>>,
    {
        self.kernel.borrow().constant_is_registered(handle)
    }

    #[inline]
    pub fn term_register_variable<T, U>(
        &self,
        name: T,
        tau: U,
    ) -> Result<Handle<tags::Term>, KernelErrorCode>
    where
        T: Into<Name>,
        U: Into<Handle<tags::Type>> + Clone,
    {
        self.kernel.borrow_mut().term_register_variable(name, tau)
    }

    #[inline]
    pub fn term_register_constant<T, U, V>(
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

    #[inline]
    pub fn term_register_application<T, U>(
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

    #[inline]
    pub fn term_register_lambda<T, U, V>(
        &self,
        name: T,
        tau: U,
        body: V,
    ) -> Result<Handle<tags::Term>, KernelErrorCode>
    where
        T: Into<Name>,
        U: Into<Handle<tags::Type>> + Clone,
        V: Into<Handle<tags::Term>> + Clone,
    {
        self.kernel
            .borrow_mut()
            .term_register_lambda(name, tau, body)
    }

    #[inline]
    pub fn term_register_negation<T>(&self, body: T) -> Result<Handle<tags::Term>, KernelErrorCode>
    where
        T: Into<Handle<tags::Term>> + Clone,
    {
        self.kernel.borrow_mut().term_register_negation(body)
    }

    #[inline]
    pub fn term_register_conjunction<T, U>(
        &self,
        left: T,
        right: T,
    ) -> Result<Handle<tags::Term>, KernelErrorCode>
    where
        T: Into<Handle<tags::Term>> + Clone,
        U: Into<Handle<tags::Term>> + Clone,
    {
        self.kernel
            .borrow_mut()
            .term_register_conjunction(left, right)
    }

    #[inline]
    pub fn term_register_disjunction<T, U>(
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

    #[inline]
    pub fn term_register_implication<T, U>(
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

    #[inline]
    pub fn term_register_equality<T, U>(
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

    #[inline]
    pub fn term_register_forall<T, U, V>(
        &self,
        name: T,
        tau: U,
        body: V,
    ) -> Result<Handle<tags::Term>, KernelErrorCode>
    where
        T: Into<Name>,
        U: Into<Handle<tags::Type>> + Clone,
        V: Into<Handle<tags::Term>> + Clone,
    {
        self.kernel
            .borrow_mut()
            .term_register_forall(name, tau, body)
    }

    #[inline]
    pub fn term_register_exists<T, U, V>(
        &self,
        name: T,
        tau: U,
        body: V,
    ) -> Result<Handle<tags::Term>, KernelErrorCode>
    where
        T: Into<Name>,
        U: Into<Handle<tags::Type>> + Clone,
        V: Into<Handle<tags::Term>> + Clone,
    {
        self.kernel
            .borrow_mut()
            .term_register_exists(name, tau, body)
    }

    #[inline]
    pub fn term_split_variable<T>(
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

    #[inline]
    pub fn term_split_constant<T>(
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

    #[inline]
    pub fn term_split_application<T>(
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

    #[inline]
    pub fn term_split_lambda<T>(
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

    #[inline]
    pub fn term_split_negation<T>(&self, handle: T) -> Result<Handle<tags::Term>, KernelErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        self.kernel
            .borrow()
            .term_split_negation(handle)
            .map(|n| n.clone())
    }

    #[inline]
    pub fn term_split_conjunction<T>(
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

    #[inline]
    pub fn term_split_disjunction<T>(
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

    #[inline]
    pub fn term_split_implication<T>(
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

    #[inline]
    pub fn term_split_equality<T>(
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

    #[inline]
    pub fn term_split_forall<T>(
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

    #[inline]
    pub fn term_split_exists<T>(
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

    #[inline]
    pub fn term_test_variable<T>(&self, handle: T) -> Result<bool, KernelErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        self.kernel.borrow().term_test_variable(handle)
    }

    #[inline]
    pub fn term_test_constant<T>(&self, handle: T) -> Result<bool, KernelErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        self.kernel.borrow().term_test_constant(handle)
    }

    #[inline]
    pub fn term_test_application<T>(&self, handle: T) -> Result<bool, KernelErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        self.kernel.borrow().term_test_application(handle)
    }

    #[inline]
    pub fn term_test_lambda<T>(&self, handle: T) -> Result<bool, KernelErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        self.kernel.borrow().term_test_lambda(handle)
    }

    #[inline]
    pub fn term_test_negation<T>(&self, handle: T) -> Result<bool, KernelErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        self.kernel.borrow().term_test_negation(handle)
    }

    #[inline]
    pub fn term_test_conjunction<T>(&self, handle: T) -> Result<bool, KernelErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        self.kernel.borrow().term_test_conjunction(handle)
    }

    #[inline]
    pub fn term_test_disjunction<T>(&self, handle: T) -> Result<bool, KernelErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        self.kernel.borrow().term_test_disjunction(handle)
    }

    #[inline]
    pub fn term_test_implication<T>(&self, handle: T) -> Result<bool, KernelErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        self.kernel.borrow().term_test_implication(handle)
    }

    #[inline]
    pub fn term_test_equality<T>(&self, handle: T) -> Result<bool, KernelErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        self.kernel.borrow().term_test_equality(handle)
    }

    #[inline]
    pub fn term_test_forall<T>(&self, handle: T) -> Result<bool, KernelErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        self.kernel.borrow().term_test_forall(handle)
    }

    #[inline]
    pub fn term_test_exists<T>(&self, handle: T) -> Result<bool, KernelErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        self.kernel.borrow().term_test_exists(handle)
    }

    #[inline]
    pub fn term_free_variables<T>(
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

    #[inline]
    pub fn term_type_variables<T>(&self, handle: T) -> Result<Vec<Name>, KernelErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        self.kernel
            .borrow()
            .term_type_variables(handle)
            .map(|v| v.iter().cloned().cloned().collect())
    }

    #[inline]
    pub fn term_substitution<T, U, V>(
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

    #[inline]
    pub fn term_type_substitution<T, U, V>(
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

    #[inline]
    pub fn term_type_infer<T>(&self, handle: T) -> Result<Handle<tags::Type>, KernelErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        self.kernel.borrow_mut().term_type_infer(handle)
    }

    #[inline]
    pub fn term_type_is_proposition<T>(&self, handle: T) -> Result<bool, KernelErrorCode>
    where
        T: Borrow<Handle<tags::Term>>,
    {
        self.kernel.borrow_mut().term_type_is_proposition(handle)
    }
}

////////////////////////////////////////////////////////////////////////////////
// Signature checking.
////////////////////////////////////////////////////////////////////////////////

/// Checks the signature of the `TypeFormer.Resolve` ABI function.
#[inline]
fn check_type_former_resolve_signature(signature: &Signature) -> bool {
    signature.params() == &[ValueType::I64, ValueType::I32]
        && signature.return_type() == Some(ValueType::I32)
}

/// Checks the signature of the `TypeFormer.Register` ABI function.
#[inline]
fn check_type_former_register_signature(signature: &Signature) -> bool {
    signature.params() == &[ValueType::I64, ValueType::I32]
        && signature.return_type() == Some(ValueType::I32)
}

/// Checks the signature of the `TypeFormer.IsRegistered` ABI function.
#[inline]
fn check_type_former_is_registered_signature(signature: &Signature) -> bool {
    signature.params() == &[ValueType::I64] && signature.return_type() == Some(ValueType::I32)
}

/// Checks the signature of the `Type.Register.Variable` ABI function.
#[inline]
fn check_type_register_variable_signature(signature: &Signature) -> bool {
    signature.params() == &[ValueType::I64] && signature.return_type() == Some(ValueType::I64)
}

/// Checks the signature of the `Type.Register.Combination` ABI function.
#[inline]
fn check_type_register_combination_signature(signature: &Signature) -> bool {
    signature.params() == &[ValueType::I64] && signature.return_type() == Some(ValueType::I64)
}

/// Checks the signature of the `Type.Register.Function` ABI function.
#[inline]
fn check_type_register_function_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Type.IsRegistered` ABI function.
#[inline]
fn check_type_is_registered_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Type.Split.Variable` ABI function.
#[inline]
fn check_type_split_variable_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Type.Split.Combination` ABI function.
#[inline]
fn check_type_split_combination_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Type.Split.Function` ABI function.
#[inline]
fn check_type_split_function_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Type.Test.Variable` ABI function.
#[inline]
fn check_type_test_variable_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Type.Test.Combination` ABI function.
#[inline]
fn check_type_test_combination_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Type.Test.Function` ABI function.
#[inline]
fn check_type_test_function_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Type.FTV` ABI function.
#[inline]
fn check_type_ftv_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Type.Substitute` ABI function.
#[inline]
fn check_type_substitute_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Constant.Register` ABI function.
#[inline]
fn check_constant_register_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Constant.Resolve` ABI function.
#[inline]
fn check_constant_resolve_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Constant.IsRegistered` ABI function.
#[inline]
fn check_constant_is_registered_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Term.Register.Variable` ABI function.
#[inline]
fn check_term_register_variable_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Term.Register.Constant` ABI function.
#[inline]
fn check_term_register_constant_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Term.Register.Application` ABI function.
#[inline]
fn check_term_register_application_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Term.Register.Lambda` ABI function.
#[inline]
fn check_term_register_lambda_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Term.Register.Negation` ABI function.
#[inline]
fn check_term_register_negation_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Term.Register.Conjunction` ABI function.
#[inline]
fn check_term_register_conjunction_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Term.Register.Disjunction` ABI function.
#[inline]
fn check_term_register_disjunction_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Term.Register.Implication` ABI function.
#[inline]
fn check_term_register_implication_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Term.Register.Equality` ABI function.
#[inline]
fn check_term_register_equality_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Term.Register.Forall` ABI function.
#[inline]
fn check_term_register_forall_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Term.Register.Exists` ABI function.
#[inline]
fn check_term_register_exists_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Term.Split.Variable` ABI function.
#[inline]
fn check_term_split_variable_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Term.Split.Constant` ABI function.
#[inline]
fn check_term_split_constant_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Term.Split.Application` ABI function.
#[inline]
fn check_term_split_application_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Term.Split.Lambda` ABI function.
#[inline]
fn check_term_split_lambda_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Term.Split.Negation` ABI function.
#[inline]
fn check_term_split_negation_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Term.Split.Conjunction` ABI function.
#[inline]
fn check_term_split_conjunction_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Term.Split.Disjunction` ABI function.
#[inline]
fn check_term_split_disjunction_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Term.Split.Implication` ABI function.
#[inline]
fn check_term_split_implication_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Term.Split.Equality` ABI function.
#[inline]
fn check_term_split_equality_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Term.Split.Forall` ABI function.
#[inline]
fn check_term_split_forall_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Term.Split.Exists` ABI function.
#[inline]
fn check_term_split_exists_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Term.Test.Variable` ABI function.
#[inline]
fn check_term_test_variable_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Term.Test.Constant` ABI function.
#[inline]
fn check_term_test_constant_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Term.Test.Application` ABI function.
#[inline]
fn check_term_test_application_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Term.Test.Lambda` ABI function.
#[inline]
fn check_term_test_lambda_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Term.Test.Negation` ABI function.
#[inline]
fn check_term_test_negation_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Term.Test.Conjunction` ABI function.
#[inline]
fn check_term_test_conjunction_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Term.Test.Disjunction` ABI function.
#[inline]
fn check_term_test_disjunction_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Term.Test.Implication` ABI function.
#[inline]
fn check_term_test_implication_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Term.Test.Equality` ABI function.
#[inline]
fn check_term_test_equality_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Term.Test.Forall` ABI function.
#[inline]
fn check_term_test_forall_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Term.Test.Exists` ABI function.
#[inline]
fn check_term_test_exists_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Term.FV` ABI function.
#[inline]
fn check_term_fv_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Term.Substitution` ABI function.
#[inline]
fn check_term_substitution_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Term.Type.Variables` ABI function.
#[inline]
fn check_term_type_variables_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Term.Type.Substitution` ABI function.
#[inline]
fn check_term_type_substitution_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Term.Type.Infer` ABI function.
#[inline]
fn check_term_type_infer_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Term.Type.IsProposition` ABI function.
#[inline]
fn check_term_type_is_proposition_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Theorem.IsRegistered` ABI function.
#[inline]
fn check_theorem_is_registered_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Theorem.Register.Reflexivity` ABI function.
#[inline]
fn check_theorem_register_reflexivity_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Theorem.Register.Symmetry` ABI function.
#[inline]
fn check_theorem_register_symmetry_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Theorem.Register.Transitivity` ABI function.
#[inline]
fn check_theorem_register_transitivity_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Theorem.Register.Application` ABI function.
#[inline]
fn check_theorem_register_application_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Theorem.Register.Lambda` ABI function.
#[inline]
fn check_theorem_register_lambda_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Theorem.Register.Beta` ABI function.
#[inline]
fn check_theorem_register_beta_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Theorem.Register.Eta` ABI function.
#[inline]
fn check_theorem_register_eta_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Theorem.Register.TruthIntroduction` ABI function.
#[inline]
fn check_theorem_register_truth_introduction_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Theorem.Register.FalsityElimination` ABI function.
#[inline]
fn check_theorem_register_falsity_elimination_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Theorem.Register.ConjunctionIntroduction` ABI function.
#[inline]
fn check_theorem_register_conjunction_introduction_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Theorem.Register.ConjunctionLeftElimination` ABI function.
#[inline]
fn check_theorem_register_conjunction_left_elimination_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Theorem.Register.ConjunctionRightElimination` ABI function.
#[inline]
fn check_theorem_register_conjunction_right_elimination_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Theorem.Register.DisjunctionElimination` ABI function.
#[inline]
fn check_theorem_register_disjunction_elimination_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Theorem.Register.DisjunctionLeftIntroduction` ABI function.
#[inline]
fn check_theorem_register_disjunction_left_introduction_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Theorem.Register.DisjunctionRightIntroduction` ABI function.
#[inline]
fn check_theorem_register_disjunction_right_introduction_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Theorem.Register.ImplicationIntroduction` ABI function.
#[inline]
fn check_theorem_register_implication_introduction_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Theorem.Register.ImplicationElimination` ABI function.
#[inline]
fn check_theorem_register_implication_elimination_signature(signature: &Signature) -> bool {
    unimplemented!()
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
                let handle = args.nth::<u64>(0) as usize;
                let result_addr = args.nth::<u32>(1);

                let arity = match self.type_former_resolve(&Handle::from(handle)) {
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
                let handle = args.nth::<u64>(0) as usize;
                let result = self.type_former_is_registered(&Handle::from(handle));

                Ok(Some(RuntimeValue::I32(result.into())))
            }
            ABI_TYPE_FORMER_REGISTER_INDEX => {
                let arity = args.nth::<u64>(0) as usize;
                let result = self.type_former_register(arity);

                Ok(Some(RuntimeValue::I64(*result as i64)))
            }
            ABI_TYPE_REGISTER_VARIABLE_INDEX => {
                let name = args.nth::<u64>(0);
                let result = self.type_register_variable(name);

                Ok(Some(RuntimeValue::I64(*result as i64)))
            }
            ABI_TYPE_REGISTER_COMBINATION_INDEX => {
                let former_handle: Handle<tags::TypeFormer> =
                    Handle::from(args.nth::<u64>(0) as usize);
                let argument_base = args.nth::<u32>(1);
                let argument_length = args.nth::<u32>(2);
                let result_ptr = args.nth::<u32>(3);

                let arguments = self.read_handles(argument_base, argument_length as usize)?;

                match self.type_register_combination(former_handle, arguments) {
                    Err(e) => Ok(Some(RuntimeValue::I64(e as i64))),
                    Ok(result) => {
                        self.write_handle(result_ptr, result)?;
                        Ok(Some(RuntimeValue::I32(KernelErrorCode::Success.into())))
                    }
                }
            }
            ABI_TYPE_REGISTER_FUNCTION_INDEX => {
                let domain_handle: Handle<tags::Type> = Handle::from(args.nth::<u64>(0) as usize);
                let range_handle: Handle<tags::Type> = Handle::from(args.nth::<u64>(1) as usize);
                let result_ptr = args.nth::<u32>(2);

                match self.type_register_function(domain_handle, range_handle) {
                    Err(e) => Ok(Some(RuntimeValue::I64(e as i64))),
                    Ok(result) => {
                        self.write_handle(result_ptr, result)?;
                        Ok(Some(RuntimeValue::I32(KernelErrorCode::Success.into())))
                    }
                }
            }
            ABI_TYPE_IS_REGISTERED_INDEX => {
                let type_handle: Handle<tags::Type> = Handle::from(args.nth::<u64>(0) as usize);
                Ok(Some(RuntimeValue::I32(
                    self.type_is_registered(type_handle).into(),
                )))
            }
            ABI_TYPE_SPLIT_VARIABLE_INDEX => {
                let type_handle: Handle<tags::Type> = Handle::from(args.nth::<u64>(0) as usize);
                let result_ptr = args.nth::<u32>(1);

                match self.type_split_variable(type_handle) {
                    Err(e) => Ok(Some(RuntimeValue::I64(e as i64))),
                    Ok(result) => {
                        self.write_u64(result_ptr, result)?;
                        Ok(Some(RuntimeValue::I32(KernelErrorCode::Success.into())))
                    }
                }
            }
            ABI_TYPE_SPLIT_COMBINATION_INDEX => {
                let type_handle: Handle<tags::Type> = Handle::from(args.nth::<u64>(0) as usize);
                let former_result_ptr = args.nth::<u32>(1);
                let arguments_result_ptr = args.nth::<u32>(2);
                let arguments_length_result_ptr = args.nth::<u32>(3);

                match self.type_split_combination(type_handle) {
                    Err(e) => Ok(Some(RuntimeValue::I64(e as i64))),
                    Ok((former_handle, arguments)) => {
                        self.write_handle(former_result_ptr, former_handle.clone())?;
                        self.write_handles(arguments_result_ptr, arguments.clone())?;
                        self.write_u32(arguments_length_result_ptr, arguments.len() as u32)?;

                        Ok(Some(RuntimeValue::I32(KernelErrorCode::Success.into())))
                    }
                }
            }
            ABI_TYPE_SPLIT_FUNCTION_INDEX => {
                let type_handle: Handle<tags::Type> = Handle::from(args.nth::<u64>(0) as usize);
                let domain_result_ptr = args.nth::<u32>(1);
                let range_result_ptr = args.nth::<u32>(2);

                match self.type_split_function(type_handle) {
                    Err(e) => Ok(Some(RuntimeValue::I64(e as i64))),
                    Ok((domain_handle, range_handle)) => {
                        self.write_handle(domain_result_ptr, domain_handle.clone())?;
                        self.write_handle(range_result_ptr, range_handle.clone())?;

                        Ok(Some(RuntimeValue::I32(KernelErrorCode::Success.into())))
                    }
                }
            }
            ABI_TYPE_TEST_VARIABLE_INDEX => {
                let type_handle: Handle<tags::Type> = Handle::from(args.nth::<u64>(0) as usize);
                let result_ptr = args.nth::<u32>(1);

                match self.type_test_variable(type_handle) {
                    Err(e) => Ok(Some(RuntimeValue::I64(e as i64))),
                    Ok(result) => {
                        self.write_i32(result_ptr, result)?;
                        Ok(Some(RuntimeValue::I32(KernelErrorCode::Success.into())))
                    }
                }
            }
            ABI_TYPE_TEST_COMBINATION_INDEX => {
                let type_handle: Handle<tags::Type> = Handle::from(args.nth::<u64>(0) as usize);
                let result_ptr = args.nth::<u32>(1);

                match self.type_test_combination(type_handle) {
                    Err(e) => Ok(Some(RuntimeValue::I64(e as i64))),
                    Ok(result) => {
                        self.write_i32(result_ptr, result)?;
                        Ok(Some(RuntimeValue::I32(KernelErrorCode::Success.into())))
                    }
                }
            }
            ABI_TYPE_TEST_FUNCTION_INDEX => {
                let type_handle: Handle<tags::Type> = Handle::from(args.nth::<u64>(0) as usize);
                let result_ptr = args.nth::<u32>(1);

                match self.type_test_function(type_handle) {
                    Err(e) => Ok(Some(RuntimeValue::I64(e as i64))),
                    Ok(result) => {
                        self.write_i32(result_ptr, result)?;
                        Ok(Some(RuntimeValue::I32(KernelErrorCode::Success.into())))
                    }
                }
            }
            ABI_TYPE_VARIABLES_INDEX => {
                let type_handle: Handle<tags::Type> = Handle::from(args.nth::<u64>(0) as usize);
                let variable_result_ptr = args.nth::<u32>(1);
                let variable_len_ptr = args.nth::<u32>(2);

                let result = self.type_variables(type_handle);

                match result {
                    Err(e) => Ok(Some(RuntimeValue::I64(e as i64))),
                    Ok(result) => {
                        self.write_u64s(variable_result_ptr, result.clone())?;
                        self.write_u32(variable_len_ptr, result.len() as u32)?;

                        Ok(Some(RuntimeValue::I32(KernelErrorCode::Success.into())))
                    }
                }
            }
            ABI_TYPE_SUBSTITUTE_INDEX => unimplemented!(),
            ABI_CONSTANT_REGISTER_INDEX => unimplemented!(),
            ABI_CONSTANT_IS_REGISTERED_INDEX => unimplemented!(),
            ABI_CONSTANT_RESOLVE_INDEX => unimplemented!(),
            ABI_TERM_REGISTER_VARIABLE_INDEX => unimplemented!(),
            ABI_TERM_REGISTER_CONSTANT_INDEX => unimplemented!(),
            ABI_TERM_REGISTER_APPLICATION_INDEX => unimplemented!(),
            ABI_TERM_REGISTER_LAMBDA_INDEX => unimplemented!(),
            ABI_TERM_REGISTER_NEGATION_INDEX => unimplemented!(),
            ABI_TERM_REGISTER_CONJUNCTION_INDEX => unimplemented!(),
            ABI_TERM_REGISTER_DISJUNCTION_INDEX => unimplemented!(),
            ABI_TERM_REGISTER_IMPLICATION_INDEX => unimplemented!(),
            ABI_TERM_REGISTER_EQUALITY_INDEX => unimplemented!(),
            ABI_TERM_REGISTER_FORALL_INDEX => unimplemented!(),
            ABI_TERM_REGISTER_EXISTS_INDEX => unimplemented!(),
            ABI_TERM_SPLIT_VARIABLE_INDEX => unimplemented!(),
            ABI_TERM_SPLIT_CONSTANT_INDEX => unimplemented!(),
            ABI_TERM_SPLIT_APPLICATION_INDEX => unimplemented!(),
            ABI_TERM_SPLIT_LAMBDA_INDEX => unimplemented!(),
            ABI_TERM_SPLIT_NEGATION_INDEX => unimplemented!(),
            ABI_TERM_SPLIT_CONJUNCTION_INDEX => unimplemented!(),
            ABI_TERM_SPLIT_DISJUNCTION_INDEX => unimplemented!(),
            ABI_TERM_SPLIT_IMPLICATION_INDEX => unimplemented!(),
            ABI_TERM_SPLIT_EQUALITY_INDEX => unimplemented!(),
            ABI_TERM_SPLIT_FORALL_INDEX => unimplemented!(),
            ABI_TERM_SPLIT_EXISTS_INDEX => unimplemented!(),
            ABI_TERM_TEST_VARIABLE_INDEX => unimplemented!(),
            ABI_TERM_TEST_CONSTANT_INDEX => unimplemented!(),
            ABI_TERM_TEST_APPLICATION_INDEX => unimplemented!(),
            ABI_TERM_TEST_LAMBDA_INDEX => unimplemented!(),
            ABI_TERM_TEST_NEGATION_INDEX => unimplemented!(),
            ABI_TERM_TEST_CONJUNCTION_INDEX => unimplemented!(),
            ABI_TERM_TEST_DISJUNCTION_INDEX => unimplemented!(),
            ABI_TERM_TEST_IMPLICATION_INDEX => unimplemented!(),
            ABI_TERM_TEST_EQUALITY_INDEX => unimplemented!(),
            ABI_TERM_TEST_FORALL_INDEX => unimplemented!(),
            ABI_TERM_TEST_EXISTS_INDEX => unimplemented!(),
            ABI_TERM_FV_INDEX => unimplemented!(),
            ABI_TERM_SUBSTITUTION_INDEX => unimplemented!(),
            ABI_TERM_TYPE_VARIABLES_INDEX => unimplemented!(),
            ABI_TERM_TYPE_SUBSTITUTION_INDEX => unimplemented!(),
            ABI_TERM_TYPE_INFER_INDEX => unimplemented!(),
            ABI_TERM_TYPE_IS_PROPOSITION_INDEX => unimplemented!(),
            _otherwise => Err(host_trap(RuntimeTrap::NoSuchFunction)),
        }
    }
}

/// Maps an ABI host-call to its associated host-call number.  Also checks that
/// the function's signature is as expected, otherwise produces a runtime error
/// that is reported back to the WASM program.
impl ModuleImportResolver for WasmiRuntimeState {
    fn resolve_func(&self, field_name: &str, signature: &Signature) -> Result<FuncRef, WasmiError> {
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
            ABI_TERM_FV_NAME => {
                if !check_term_fv_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TERM_FV_INDEX,
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
                if !check_theorem_register_truth_introduction_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_THEOREM_REGISTER_TRUTH_INTRODUCTION_INDEX,
                ))
            }
            ABI_THEOREM_REGISTER_FALSITY_ELIMINATION_NAME => {
                if !check_theorem_register_falsity_elimination_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_THEOREM_REGISTER_FALSITY_ELIMINATION_INDEX,
                ))
            }
            ABI_THEOREM_REGISTER_CONJUNCTION_INTRODUCTION_NAME => {
                if !check_theorem_register_conjunction_introduction_signature(signature) {
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
                if !check_theorem_register_disjunction_elimination_signature(signature) {
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
                if !check_theorem_register_implication_introduction_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_THEOREM_REGISTER_IMPLICATION_INTRODUCTION_INDEX,
                ))
            }
            ABI_THEOREM_REGISTER_IMPLICATION_ELIMINATION_NAME => {
                if !check_theorem_register_implication_elimination_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_THEOREM_REGISTER_IMPLICATION_ELIMINATION_INDEX,
                ))
            }
            ABI_THEOREM_REGISTER_IFF_INTRODUCTION_NAME => unimplemented!(),
            ABI_THEOREM_REGISTER_IFF_LEFT_ELIMINATION_NAME => unimplemented!(),
            ABI_THEOREM_REGISTER_NEGATION_INTRODUCTION_NAME => unimplemented!(),
            ABI_THEOREM_REGISTER_NEGATION_ELIMINATION_NAME => unimplemented!(),
            ABI_THEOREM_REGISTER_FORALL_INTRODUCTION_NAME => unimplemented!(),
            ABI_THEOREM_REGISTER_FORALL_ELIMINATION_NAME => unimplemented!(),
            ABI_THEOREM_REGISTER_EXISTS_ELIMINATION_NAME => unimplemented!(),
            ABI_THEOREM_REGISTER_EXISTS_INTRODUCTION_NAME => unimplemented!(),
            ABI_THEOREM_SPLIT_CONCLUSION_NAME => unimplemented!(),
            ABI_THEOREM_SPLIT_HYPOTHESES_NAME => unimplemented!(),
            _otherwise => Err(host_error(KernelErrorCode::NoSuchFunction)),
        }
    }
}
