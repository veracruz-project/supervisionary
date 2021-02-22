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
    fmt::{Display, Error as DisplayError, Formatter},
    mem::size_of,
};

use crate::kernel::{
    error_code::ErrorCode as KernelErrorCode, handle::Handle,
    runtime_state::RuntimeState as KernelRuntimeState,
};

use crate::kernel::name::Name;
use byteorder::{ByteOrder, LittleEndian};
use wasmi::{
    Error as WasmiError, Externals, FuncInstance, FuncRef, HostError, MemoryInstance,
    ModuleImportResolver, RuntimeArgs, RuntimeValue, Signature, Trap, TrapKind, ValueType,
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

/// The name of the `TypeFormer.Handle.Resolve` ABI call.
pub const ABI_TYPE_FORMER_HANDLE_RESOLVE_NAME: &'static str = "__type_former_handle_resolve";
/// The name of the `TypeFormer.Handle.IsRegistered` ABI call.
pub const ABI_TYPE_FORMER_HANDLE_IS_REGISTERED_NAME: &'static str =
    "__type_former_handle_is_registered";
/// The name of the `TypeFormer.Handle.Register` ABI call.
pub const ABI_TYPE_FORMER_HANDLE_REGISTER_NAME: &'static str = "__type_former_handle_register";

/// The host-call number of the `TypeFormer.Handle.Resolve` ABI call.
pub const ABI_TYPE_FORMER_HANDLE_RESOLVE_INDEX: usize = 0;
/// The host-call number of the `TypeFormer.Handle.IsRegistered` ABI call.
pub const ABI_TYPE_FORMER_HANDLE_IS_REGISTERED_INDEX: usize = 1;
/// The host-call number of the `TypeFormer.Handle.Register` ABI call.
pub const ABI_TYPE_FORMER_HANDLE_REGISTER_INDEX: usize = 2;

/* Type-related calls. */

/// The name of the `Type.Handle.IsRegistered` ABI call.
pub const ABI_TYPE_HANDLE_IS_REGISTERED_NAME: &'static str = "__type_handle_is_registered";
/// The name of the `Type.Handle.Register.Variable` ABI call.
pub const ABI_TYPE_HANDLE_REGISTER_VARIABLE_NAME: &'static str = "__type_handle_register_variable";
/// The name of the `Type.Handle.Register.Combination` ABI call.
pub const ABI_TYPE_HANDLE_REGISTER_COMBINATION_NAME: &'static str =
    "__type_handle_register_combination_name";
/// The name of the `Type.Handle.Register.Function` ABI call.
pub const ABI_TYPE_HANDLE_REGISTER_FUNCTION_NAME: &'static str =
    "__type_handle_register_function_name";

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

/// The name of the `Type.FTV` ABI call.
pub const ABI_TYPE_FTV_NAME: &'static str = "__type_ftv";
/// The name of the `Type.Substitute` ABI call.
pub const ABI_TYPE_SUBSTITUTE_NAME: &'static str = "__type_substitute";

/// The host-call number of the `Type.Handle.IsRegistered` ABI call.
pub const ABI_TYPE_HANDLE_IS_REGISTERED_INDEX: usize = 3;
/// The host-call number of the `Type.Handle.Register.Variable` ABI call.
pub const ABI_TYPE_HANDLE_REGISTER_VARIABLE_INDEX: usize = 4;
/// The host-call number of the `Type.Handle.Register.Combination` ABI call.
pub const ABI_TYPE_HANDLE_REGISTER_COMBINATION_INDEX: usize = 5;
/// The host-call number of the `Type.Handle.Register.Function` ABI call.
pub const ABI_TYPE_HANDLE_REGISTER_FUNCTION_INDEX: usize = 6;

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

/// The name of the `Constant.Handle.Resolve` ABI call.
pub const ABI_CONSTANT_HANDLE_RESOLVE_NAME: &'static str = "__constant_handle_resolve";
/// The name of the `Constant.Handle.IsRegistered` ABI call.
pub const ABI_CONSTANT_HANDLE_IS_REGISTERED_NAME: &'static str = "__constant_handle_is_registered";
/// The name of the `Constant.Handle.Register` ABI call.
pub const ABI_CONSTANT_HANDLE_REGISTER_NAME: &'static str = "__constant_handle_register";

/// The host-call number of the `Constant.Handle.Register` ABI call.
pub const ABI_CONSTANT_HANDLE_RESOLVE_INDEX: usize = 15;
/// The host-call number of the `Constant.Handle.IsRegistered` ABI call.
pub const ABI_CONSTANT_HANDLE_IS_REGISTERED_INDEX: usize = 16;
/// The host-call number of the `Constant.Handle.Register` ABI call.
pub const ABI_CONSTANT_HANDLE_REGISTER_INDEX: usize = 17;

/* Term-related calls. */

/// The name of the `Term.Handle.Register.Variable` ABI call.
pub const ABI_TERM_HANDLE_REGISTER_VARIABLE_NAME: &'static str = "__term_handle_register_variable";
/// The name of the `Term.Handle.Register.Constant` ABI call.
pub const ABI_TERM_HANDLE_REGISTER_CONSTANT_NAME: &'static str = "__term_handle_register_constant";
/// The name of the `Term.Handle.Register.Application` ABI call.
pub const ABI_TERM_HANDLE_REGISTER_APPLICATION_NAME: &'static str =
    "__term_handle_register_application";
/// The name of the `Term.Handle.Register.Lambda` ABI call.
pub const ABI_TERM_HANDLE_REGISTER_LAMBDA_NAME: &'static str = "__term_handle_register_lambda";
/// The name of the `Term.Handle.Register.Negation` ABI call.
pub const ABI_TERM_HANDLE_REGISTER_NEGATION_NAME: &'static str = "__term_handle_register_negation";
/// The name of the `Term.Handle.Register.Conjunction` ABI call.
pub const ABI_TERM_HANDLE_REGISTER_CONJUNCTION_NAME: &'static str =
    "__term_handle_register_conjunction";
/// The name of the `Term.Handle.Register.Disjunction` ABI call.
pub const ABI_TERM_HANDLE_REGISTER_DISJUNCTION_NAME: &'static str =
    "__term_handle_register_disjunction";
/// The name of the `Term.Handle.Register.Implication` ABI call.
pub const ABI_TERM_HANDLE_REGISTER_IMPLICATION_NAME: &'static str =
    "__term_handle_register_implication";
/// The name of the `Term.Handle.Register.Equality` ABI call.
pub const ABI_TERM_HANDLE_REGISTER_EQUALITY_NAME: &'static str = "__term_handle_register_equality";
/// The name of the `Term.Handle.Register.Forall` ABI call.
pub const ABI_TERM_HANDLE_REGISTER_FORALL_NAME: &'static str = "__term_handle_register_forall";
/// The name of the `Term.Handle.Register.Exists` ABI call.
pub const ABI_TERM_HANDLE_REGISTER_EXISTS_NAME: &'static str = "__term_handle_register_exists";

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

/// The host-call number of the `Term.Handle.Register.Variable` ABI call.
pub const ABI_TERM_HANDLE_REGISTER_VARIABLE_INDEX: usize = 18;
/// The host-call number of the `Term.Handle.Register.Constant` ABI call.
pub const ABI_TERM_HANDLE_REGISTER_CONSTANT_INDEX: usize = 19;
/// The host-call number of the `Term.Handle.Register.Application` ABI call.
pub const ABI_TERM_HANDLE_REGISTER_APPLICATION_INDEX: usize = 20;
/// The host-call number of the `Term.Handle.Register.Lambda` ABI call.
pub const ABI_TERM_HANDLE_REGISTER_LAMBDA_INDEX: usize = 21;
/// The host-call number of the `Term.Handle.Register.Negation` ABI call.
pub const ABI_TERM_HANDLE_REGISTER_NEGATION_INDEX: usize = 22;
/// The host-call number of the `Term.Handle.Register.Conjunction` ABI call.
pub const ABI_TERM_HANDLE_REGISTER_CONJUNCTION_INDEX: usize = 23;
/// The host-call number of the `Term.Handle.Register.Disjunction` ABI call.
pub const ABI_TERM_HANDLE_REGISTER_DISJUNCTION_INDEX: usize = 24;
/// The host-call number of the `Term.Handle.Register.Implication` ABI call.
pub const ABI_TERM_HANDLE_REGISTER_IMPLICATION_INDEX: usize = 25;
/// The host-call number of the `Term.Handle.Register.Equality` ABI call.
pub const ABI_TERM_HANDLE_REGISTER_EQUALITY_INDEX: usize = 26;
/// The host-call number of the `Term.Handle.Register.Forall` ABI call.
pub const ABI_TERM_HANDLE_REGISTER_FORALL_INDEX: usize = 27;
/// The host-call number of the `Term.Handle.Register.Exists` ABI call.
pub const ABI_TERM_HANDLE_REGISTER_EXISTS_INDEX: usize = 28;

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

pub const ABI_THEOREM_HANDLE_REGISTER_EQUALITY_REFLEXIVITY_NAME: &'static str =
    "__theorem_handle_register_equality_reflexivity";
pub const ABI_THEOREM_HANDLE_REGISTER_EQUALITY_SYMMETRY_NAME: &'static str =
    "__theorem_handle_register_equality_symmetry";
pub const ABI_THEOREM_HANDLE_REGISTER_EQUALITY_TRANSITIVITY_NAME: &'static str =
    "__theorem_handle_register_equality_transitivity";

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
    kernel: KernelRuntimeState,
    /// The memory instance of the executing WASM guest program.
    memory: Option<MemoryInstance>,
}

impl Default for WasmiRuntimeState {
    #[inline]
    fn default() -> Self {
        Self {
            kernel: Default::default(),
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

    /// Returns a reference to the WASM guest's memory module.
    #[inline]
    pub fn memory(&self) -> Option<&MemoryInstance> {
        self.memory.as_ref()
    }

    /// Returns `true` iff the WASM guest's memory module has been registered.
    #[inline]
    pub fn is_memory_registered(&self) -> bool {
        self.memory.is_some()
    }

    /// Registers the WASM guest's memory module with the runtime state.
    #[inline]
    pub fn set_memory(&mut self, instance: MemoryInstance) -> &mut Self {
        self.memory = Some(instance);
        self
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
    pub fn write_u32<T, U>(&mut self, address: T, value: U) -> Result<(), RuntimeTrap>
    where
        T: Into<Address>,
        U: Into<u32>,
    {
        let memory = match &self.memory {
            None => return Err(RuntimeTrap::MemoryNotRegistered),
            Some(memory) => memory,
        };

        let mut buffer = Vec::new();
        LittleEndian::write_u32(&mut buffer, value.into());

        memory
            .set(address.into(), &buffer)
            .map_err(|_e| RuntimeTrap::MemoryWriteFailed)
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
    pub fn write_i32<T, U>(&mut self, address: T, value: U) -> Result<(), RuntimeTrap>
    where
        T: Into<Address>,
        U: Into<i32>,
    {
        let memory = match &self.memory {
            None => return Err(RuntimeTrap::MemoryNotRegistered),
            Some(memory) => memory,
        };

        let mut buffer = Vec::new();
        LittleEndian::write_i32(&mut buffer, value.into());

        memory
            .set(address.into(), &buffer)
            .map_err(|_e| RuntimeTrap::MemoryWriteFailed)
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
    pub fn write_u64<T, U>(&mut self, address: T, value: U) -> Result<(), RuntimeTrap>
    where
        T: Into<Address>,
        U: Into<u64>,
    {
        let memory = match &self.memory {
            None => return Err(RuntimeTrap::MemoryNotRegistered),
            Some(memory) => memory,
        };

        let mut buffer = Vec::new();
        LittleEndian::write_u64(&mut buffer, value.into());

        memory
            .set(address.into(), &buffer)
            .map_err(|_e| RuntimeTrap::MemoryWriteFailed)
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
    pub fn write_i64<T, U>(&mut self, address: T, value: U) -> Result<(), RuntimeTrap>
    where
        T: Into<Address>,
        U: Into<i64>,
    {
        let memory = match &self.memory {
            None => return Err(RuntimeTrap::MemoryNotRegistered),
            Some(memory) => memory,
        };

        let mut buffer = Vec::new();
        LittleEndian::write_i64(&mut buffer, value.into());

        memory
            .set(address.into(), &buffer)
            .map_err(|_e| RuntimeTrap::MemoryWriteFailed)
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
            .get(address.into(), byte_count.into())
            .map_err(|_e| RuntimeTrap::MemoryReadFailed)?;

        Ok(bytes)
    }

    pub fn read_u32<T>(&self, address: T) -> Result<u32, RuntimeTrap>
    where
        T: Into<Address>,
    {
        let buffer = self.read_bytes(address, size_of::<u32>())?;
        Ok(LittleEndian::read_u32(&buffer))
    }

    pub fn read_u64<T>(&self, address: T) -> Result<u64, RuntimeTrap>
    where
        T: Into<Address>,
    {
        let buffer = self.read_bytes(address, size_of::<u64>())?;
        Ok(LittleEndian::read_u64(&buffer))
    }

    pub fn read_i32<T>(&self, address: T) -> Result<i32, RuntimeTrap>
    where
        T: Into<Address>,
    {
        let buffer = self.read_bytes(address, size_of::<i32>())?;
        Ok(LittleEndian::read_i32(&buffer))
    }

    pub fn read_i64<T>(&self, address: T) -> Result<i64, RuntimeTrap>
    where
        T: Into<Address>,
    {
        let buffer = self.read_bytes(address, size_of::<i32>())?;
        Ok(LittleEndian::read_i64(&buffer))
    }

    ////////////////////////////////////////////////////////////////////////////
    // Kernel-related functionality.
    ////////////////////////////////////////////////////////////////////////////

    #[inline]
    pub fn type_former_handle_resolve(&self, handle: &Handle) -> Option<&usize> {
        self.kernel.type_former_handle_resolve(handle)
    }

    #[inline]
    pub fn type_former_handle_is_registered(&self, handle: &Handle) -> bool {
        self.kernel.type_former_handle_is_registered(handle)
    }

    #[inline]
    pub fn type_former_handle_register<T>(&mut self, arity: T) -> Handle
    where
        T: Into<usize>,
    {
        self.kernel.type_former_handle_register(arity)
    }

    #[inline]
    pub fn type_handle_register_variable<T>(&mut self, name: T) -> Handle
    where
        T: Into<Name>,
    {
        self.kernel.type_handle_register_variable(name)
    }

    #[inline]
    pub fn type_handle_register_combination<T>(
        &mut self,
        type_former: T,
        arguments: Vec<T>,
    ) -> Result<Handle, KernelErrorCode>
    where
        T: Into<Handle> + Clone,
    {
        self.kernel.type_handle_register_combination(
            type_former.into(),
            arguments.iter().cloned().map(|a| a.into()).collect(),
        )
    }

    #[inline]
    pub fn type_handle_register_function<T>(
        &mut self,
        domain: T,
        range: T,
    ) -> Result<Handle, KernelErrorCode>
    where
        T: Into<Handle>,
    {
        self.kernel
            .type_handle_register_function(domain.into(), range.into())
    }

    #[inline]
    pub fn type_handle_is_registered<'a, T>(&'a self, handle: T) -> bool
    where
        T: Into<&'a Handle>,
    {
        self.kernel.type_handle_is_registered(handle.into())
    }

    #[inline]
    pub fn type_split_variable<'a, T>(&'a self, handle: T) -> Result<&Name, KernelErrorCode>
    where
        T: Into<&'a Handle>,
    {
        self.kernel.type_split_variable(handle.into())
    }

    #[inline]
    pub fn type_split_combination<'a, T>(
        &'a self,
        handle: T,
    ) -> Result<(&Handle, &Vec<Handle>), KernelErrorCode>
    where
        T: Into<&'a Handle>,
    {
        self.kernel.type_split_combination(handle.into())
    }

    #[inline]
    pub fn type_split_function<'a, T>(
        &'a self,
        handle: T,
    ) -> Result<(&Handle, &Handle), KernelErrorCode>
    where
        T: Into<&'a Handle>,
    {
        self.kernel.type_split_function(handle.into())
    }

    #[inline]
    pub fn type_test_is_variable<'a, T>(&'a self, handle: T) -> Result<bool, KernelErrorCode>
    where
        T: Into<&'a Handle>,
    {
        self.kernel.type_test_is_variable(handle.into())
    }

    #[inline]
    pub fn type_test_is_combination<'a, T>(&'a self, handle: T) -> Result<bool, KernelErrorCode>
    where
        T: Into<&'a Handle>,
    {
        self.kernel.type_test_is_combination(handle.into())
    }

    #[inline]
    pub fn type_test_is_function<'a, T>(&'a self, handle: T) -> Result<bool, KernelErrorCode>
    where
        T: Into<&'a Handle>,
    {
        self.kernel.type_test_is_function(handle.into())
    }

    #[inline]
    pub fn type_ftv<'a, T>(&'a mut self, handle: T) -> Result<Vec<&Name>, KernelErrorCode>
    where
        T: Into<&'a Handle>,
    {
        self.kernel.type_ftv(handle.into())
    }

    #[inline]
    pub fn type_substitute<'a, T, U>(
        &'a mut self,
        handle: T,
        sigma: U,
    ) -> Result<Handle, KernelErrorCode>
    where
        T: Into<&'a Handle> + Clone,
        U: Iterator<Item = (Name, Handle)> + Clone,
    {
        self.kernel.type_substitute(handle.into(), sigma)
    }

    #[inline]
    pub fn constant_handle_register<T>(&mut self, handle: T) -> Result<Handle, KernelErrorCode>
    where
        T: Into<Handle>,
    {
        self.kernel.constant_handle_register(handle.into())
    }

    #[inline]
    pub fn constant_handle_resolve<'a, T>(&'a self, handle: T) -> Result<&Handle, KernelErrorCode>
    where
        T: Into<&'a Handle>,
    {
        self.kernel
            .constant_handle_resolve(handle.into())
            .ok_or(KernelErrorCode::NoSuchConstantRegistered)
    }

    #[inline]
    pub fn constant_handle_is_registered<'a, T>(&'a self, handle: T) -> bool
    where
        T: Into<&'a Handle>,
    {
        self.kernel.constant_handle_is_registered(handle.into())
    }
}

////////////////////////////////////////////////////////////////////////////////
// Signature checking.
////////////////////////////////////////////////////////////////////////////////

/// Checks the signature of the `TypeFormer.Handle.Resolve` ABI function.
#[inline]
fn check_type_former_handle_resolve_signature(signature: &Signature) -> bool {
    signature.params() == &[ValueType::I64, ValueType::I32]
        && signature.return_type() == Some(ValueType::I32)
}

/// Checks the signature of the `TypeFormer.Handle.Register` ABI function.
#[inline]
fn check_type_former_handle_register_signature(signature: &Signature) -> bool {
    signature.params() == &[ValueType::I64, ValueType::I32]
        && signature.return_type() == Some(ValueType::I32)
}

/// Checks the signature of the `TypeFormer.Handle.IsRegistered` ABI function.
#[inline]
fn check_type_former_handle_is_registered_signature(signature: &Signature) -> bool {
    signature.params() == &[ValueType::I64] && signature.return_type() == Some(ValueType::I32)
}

/// Checks the signature of the `Type.Handle.Register.Variable` ABI function.
#[inline]
fn check_type_handle_register_variable_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Type.Handle.Register.Combination` ABI function.
#[inline]
fn check_type_handle_register_combination_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Type.Handle.Register.Function` ABI function.
#[inline]
fn check_type_handle_register_function_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Type.Handle.IsRegistered` ABI function.
#[inline]
fn check_type_handle_is_registered_signature(signature: &Signature) -> bool {
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

/// Checks the signature of the `Constant.Handle.Register` ABI function.
#[inline]
fn check_constant_handle_register_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Constant.Handle.Resolve` ABI function.
#[inline]
fn check_constant_handle_resolve_signature(signature: &Signature) -> bool {
    unimplemented!()
}

/// Checks the signature of the `Constant.Handle.IsRegistered` ABI function.
#[inline]
fn check_constant_handle_is_registered_signature(signature: &Signature) -> bool {
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
            ABI_TYPE_FORMER_HANDLE_RESOLVE_INDEX => {
                let handle = args.nth::<u64>(0) as usize;
                let result_addr = args.nth::<u32>(1);

                let arity = match self.type_former_handle_resolve(&handle) {
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
            ABI_TYPE_FORMER_HANDLE_IS_REGISTERED_INDEX => {
                let handle = args.nth::<u64>(0) as usize;
                let result = self.type_former_handle_is_registered(&handle);

                Ok(Some(RuntimeValue::I32(result.into())))
            }
            ABI_TYPE_FORMER_HANDLE_REGISTER_INDEX => {
                let arity = args.nth::<u64>(0) as usize;
                let result = self.type_former_handle_register(arity);

                Ok(Some(RuntimeValue::I64(result as i64)))
            }
            ABI_TYPE_HANDLE_REGISTER_VARIABLE_INDEX => unimplemented!(),
            ABI_TYPE_HANDLE_REGISTER_COMBINATION_INDEX => unimplemented!(),
            ABI_TYPE_HANDLE_REGISTER_FUNCTION_INDEX => unimplemented!(),
            ABI_TYPE_HANDLE_IS_REGISTERED_INDEX => unimplemented!(),
            ABI_TYPE_SPLIT_VARIABLE_INDEX => unimplemented!(),
            ABI_TYPE_SPLIT_COMBINATION_INDEX => unimplemented!(),
            ABI_TYPE_SPLIT_FUNCTION_INDEX => unimplemented!(),
            ABI_TYPE_TEST_VARIABLE_INDEX => unimplemented!(),
            ABI_TYPE_TEST_COMBINATION_INDEX => unimplemented!(),
            ABI_TYPE_TEST_FUNCTION_INDEX => unimplemented!(),
            ABI_TYPE_VARIABLES_INDEX => unimplemented!(),
            ABI_TYPE_SUBSTITUTE_INDEX => unimplemented!(),
            ABI_CONSTANT_HANDLE_REGISTER_INDEX => unimplemented!(),
            ABI_CONSTANT_HANDLE_IS_REGISTERED_INDEX => unimplemented!(),
            ABI_CONSTANT_HANDLE_RESOLVE_INDEX => unimplemented!(),
            ABI_TERM_HANDLE_REGISTER_VARIABLE_INDEX => unimplemented!(),
            ABI_TERM_HANDLE_REGISTER_CONSTANT_INDEX => unimplemented!(),
            ABI_TERM_HANDLE_REGISTER_APPLICATION_INDEX => unimplemented!(),
            ABI_TERM_HANDLE_REGISTER_LAMBDA_INDEX => unimplemented!(),
            ABI_TERM_HANDLE_REGISTER_NEGATION_INDEX => unimplemented!(),
            ABI_TERM_HANDLE_REGISTER_CONJUNCTION_INDEX => unimplemented!(),
            ABI_TERM_HANDLE_REGISTER_DISJUNCTION_INDEX => unimplemented!(),
            ABI_TERM_HANDLE_REGISTER_IMPLICATION_INDEX => unimplemented!(),
            ABI_TERM_HANDLE_REGISTER_EQUALITY_INDEX => unimplemented!(),
            ABI_TERM_HANDLE_REGISTER_FORALL_INDEX => unimplemented!(),
            ABI_TERM_HANDLE_REGISTER_EXISTS_INDEX => unimplemented!(),
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
            ABI_TYPE_FORMER_HANDLE_RESOLVE_NAME => {
                if !check_type_former_handle_resolve_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TYPE_FORMER_HANDLE_RESOLVE_INDEX,
                ))
            }
            ABI_TYPE_FORMER_HANDLE_REGISTER_NAME => {
                if !check_type_former_handle_register_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TYPE_FORMER_HANDLE_REGISTER_INDEX,
                ))
            }
            ABI_TYPE_FORMER_HANDLE_IS_REGISTERED_NAME => {
                if !check_type_former_handle_is_registered_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TYPE_FORMER_HANDLE_IS_REGISTERED_INDEX,
                ))
            }
            ABI_TYPE_HANDLE_IS_REGISTERED_NAME => {
                if !check_type_handle_is_registered_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TYPE_HANDLE_IS_REGISTERED_INDEX,
                ))
            }
            ABI_TYPE_HANDLE_REGISTER_VARIABLE_NAME => {
                if !check_type_handle_register_variable_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TYPE_HANDLE_REGISTER_VARIABLE_INDEX,
                ))
            }
            ABI_TYPE_HANDLE_REGISTER_COMBINATION_NAME => {
                if !check_type_handle_register_combination_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TYPE_HANDLE_REGISTER_COMBINATION_INDEX,
                ))
            }
            ABI_TYPE_HANDLE_REGISTER_FUNCTION_NAME => {
                if !check_type_handle_register_function_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TYPE_HANDLE_REGISTER_FUNCTION_INDEX,
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
            ABI_CONSTANT_HANDLE_RESOLVE_NAME => {
                if !check_constant_handle_resolve_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_CONSTANT_HANDLE_RESOLVE_INDEX,
                ))
            }
            ABI_CONSTANT_HANDLE_IS_REGISTERED_NAME => {
                if !check_constant_handle_is_registered_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_CONSTANT_HANDLE_IS_REGISTERED_INDEX,
                ))
            }
            ABI_CONSTANT_HANDLE_REGISTER_NAME => {
                if !check_constant_handle_register_signature(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_CONSTANT_HANDLE_REGISTER_INDEX,
                ))
            }
            ABI_TERM_HANDLE_REGISTER_VARIABLE_NAME => unimplemented!(),
            ABI_TERM_HANDLE_REGISTER_CONSTANT_NAME => unimplemented!(),
            ABI_TERM_HANDLE_REGISTER_APPLICATION_NAME => unimplemented!(),
            ABI_TERM_HANDLE_REGISTER_LAMBDA_NAME => unimplemented!(),
            ABI_TERM_HANDLE_REGISTER_NEGATION_NAME => unimplemented!(),
            ABI_TERM_HANDLE_REGISTER_CONJUNCTION_NAME => unimplemented!(),
            ABI_TERM_HANDLE_REGISTER_DISJUNCTION_NAME => unimplemented!(),
            ABI_TERM_HANDLE_REGISTER_IMPLICATION_NAME => unimplemented!(),
            ABI_TERM_HANDLE_REGISTER_EQUALITY_NAME => unimplemented!(),
            ABI_TERM_HANDLE_REGISTER_FORALL_NAME => unimplemented!(),
            ABI_TERM_HANDLE_REGISTER_EXISTS_NAME => unimplemented!(),
            ABI_TERM_SPLIT_VARIABLE_NAME => unimplemented!(),
            ABI_TERM_SPLIT_CONSTANT_NAME => unimplemented!(),
            ABI_TERM_SPLIT_APPLICATION_NAME => unimplemented!(),
            ABI_TERM_SPLIT_LAMBDA_NAME => unimplemented!(),
            ABI_TERM_SPLIT_NEGATION_NAME => unimplemented!(),
            ABI_TERM_SPLIT_CONJUNCTION_NAME => unimplemented!(),
            ABI_TERM_SPLIT_DISJUNCTION_NAME => unimplemented!(),
            ABI_TERM_SPLIT_IMPLICATION_NAME => unimplemented!(),
            ABI_TERM_SPLIT_EQUALITY_NAME => unimplemented!(),
            ABI_TERM_SPLIT_FORALL_NAME => unimplemented!(),
            ABI_TERM_SPLIT_EXISTS_NAME => unimplemented!(),
            ABI_TERM_TEST_VARIABLE_NAME => unimplemented!(),
            ABI_TERM_TEST_CONSTANT_NAME => unimplemented!(),
            ABI_TERM_TEST_APPLICATION_NAME => unimplemented!(),
            ABI_TERM_TEST_LAMBDA_NAME => unimplemented!(),
            ABI_TERM_TEST_NEGATION_NAME => unimplemented!(),
            ABI_TERM_TEST_CONJUNCTION_NAME => unimplemented!(),
            ABI_TERM_TEST_DISJUNCTION_NAME => unimplemented!(),
            ABI_TERM_TEST_IMPLICATION_NAME => unimplemented!(),
            ABI_TERM_TEST_EQUALITY_NAME => unimplemented!(),
            ABI_TERM_TEST_FORALL_NAME => unimplemented!(),
            ABI_TERM_TEST_EXISTS_NAME => unimplemented!(),
            ABI_TERM_FV_NAME => unimplemented!(),
            ABI_TERM_SUBSTITUTION_NAME => unimplemented!(),
            ABI_TERM_TYPE_VARIABLES_NAME => unimplemented!(),
            ABI_TERM_TYPE_SUBSTITUTION_NAME => unimplemented!(),
            ABI_TERM_TYPE_INFER_NAME => unimplemented!(),
            ABI_TERM_TYPE_IS_PROPOSITION_NAME => unimplemented!(),
            _otherwise => Err(host_error(KernelErrorCode::NoSuchFunction)),
        }
    }
}
