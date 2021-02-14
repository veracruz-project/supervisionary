//! # Host interface to WASM code.
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

use std::fmt::{Display, Error as DisplayError, Formatter};

use crate::kernel::{
    error_code::ErrorCode as KernelErrorCode, runtime_state::RuntimeState as KernelRuntimeState,
};

use crate::kernel::handle::Handle;
use crate::kernel::runtime_state::RuntimeState;
use wasmi::{
    Error as WasmiError, Externals, FuncInstance, FuncRef, HostError, LittleEndianConvert,
    MemoryInstance, ModuleImportResolver, RuntimeArgs, RuntimeValue, Signature, Trap, TrapKind,
    ValueType,
};

////////////////////////////////////////////////////////////////////////////////
// Miscellaneous material.
////////////////////////////////////////////////////////////////////////////////

/// A WASM address, used for reading-from and writing-to the guest WASM program
/// heap.
pub type Address = u32;

////////////////////////////////////////////////////////////////////////////////
// ABI: host-call names and numbers.
////////////////////////////////////////////////////////////////////////////////

/* Type-former related calls. */

pub const ABI_TYPE_FORMER_RESOLVE_HANDLE_NAME: &'static str = "__type_former_resolve_handle";
pub const ABI_TYPE_FORMER_IS_HANDLE_REGISTERED_NAME: &'static str =
    "__type_former_is_handle_registered";
pub const ABI_TYPE_FORMER_REGISTER_HANDLE_NAME: &'static str = "__type_former_register_handle";

pub const ABI_TYPE_FORMER_RESOLVE_HANDLE_INDEX: usize = 0;
pub const ABI_TYPE_FORMER_IS_HANDLE_REGISTERED_INDEX: usize = 1;
pub const ABI_TYPE_FORMER_REGISTER_HANDLE_INDEX: usize = 2;

/* Type-related calls. */

////////////////////////////////////////////////////////////////////////////////
// Errors and traps.
////////////////////////////////////////////////////////////////////////////////

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

    /// Writes a value to the WASM guest's memory module at a specified address.
    ///
    /// # Errors
    ///
    /// Returns `Err(RuntimeTrap::MemoryNotRegistered)` if the WASM guest's
    /// memory module has not been registered with the runtime state.
    ///
    /// Returns `Err(RuntimeTrap::MemoryWriteFailed)` if the write to memory at
    /// address, `address`, failed.
    pub fn write_memory<T, U>(&mut self, address: T, value: U) -> Result<(), RuntimeTrap>
    where
        T: Into<Address>,
        U: LittleEndianConvert,
    {
        let mut memory = match &self.memory {
            None => return Err(RuntimeTrap::MemoryNotRegistered),
            Some(memory) => memory,
        };

        let mut buffer = Vec::new();
        value.into_little_endian(&mut buffer);

        memory
            .set(address.into(), &buffer)
            .map_err(|_e| RuntimeTrap::MemoryWriteFailed)
    }

    /// Reads a value to the WASM guest's memory module at a specified address,
    /// with a given byte-count.
    ///
    /// # Errors
    ///
    /// Returns `Err(RuntimeTrap::MemoryNotRegistered)` if the WASM guest's
    /// memory module has not been registered with the runtime state.
    ///
    /// Returns `Err(RuntimeTrap::MemoryReadFailed)` if the read from memory at
    /// address, `address`, failed or if the value cannot be converted from the
    /// read slice of bytes.
    pub fn read_memory<T, U, V>(&self, address: T, byte_count: U) -> Result<V, RuntimeTrap>
    where
        T: Into<Address>,
        U: Into<usize>,
        V: LittleEndianConvert,
    {
        let memory = match &self.memory {
            None => return Err(RuntimeTrap::MemoryNotRegistered),
            Some(memory) => memory,
        };

        let bytes = memory
            .get(address.into(), byte_count.into())
            .map_err(|_e| RuntimeTrap::MemoryReadFailed)?;

        LittleEndianConvert::from_little_endian(&bytes).map_err(|_e| RuntimeTrap::MemoryReadFailed)
    }

    ////////////////////////////////////////////////////////////////////////////
    // Kernel-related functionality.
    ////////////////////////////////////////////////////////////////////////////

    #[inline]
    pub fn type_former_resolve_handle(&self, handle: &Handle) -> Option<&usize> {
        self.kernel.resolve_type_former_handle(handle)
    }

    #[inline]
    pub fn type_former_is_handle_registered(&self, handle: &Handle) -> bool {
        self.kernel.is_type_former_registered(handle)
    }

    #[inline]
    pub fn type_former_register<T>(&mut self, arity: T) -> Handle
    where
        T: Into<usize>,
    {
        self.kernel.register_type_former(arity)
    }
}

////////////////////////////////////////////////////////////////////////////////
// Signature checking.
////////////////////////////////////////////////////////////////////////////////

/// Checks the signature of the `type_former_resolve_handle()` ABI function.
///
/// # Parameters:
///
/// 1. `handle`, of type `u64`, the handle to resolve.
/// 2. `pointer`, of type `u32`, the address in the WASM heap of where the
/// result should be written.
///
/// # Return value:
///
/// `ErrorCode` of type `u32` signalling success or failure.
#[inline]
fn check_type_former_resolve_handle(signature: &Signature) -> bool {
    signature.params() == &[ValueType::I64, ValueType::I32]
        && signature.return_type() == Some(ValueType::I32)
}

#[inline]
fn check_type_former_register_handle(signature: &Signature) -> bool {
    signature.params() == &[ValueType::I64, ValueType::I32]
        && signature.return_type() == Some(ValueType::I32)
}

#[inline]
fn check_type_former_is_handle_registered(signature: &Signature) -> bool {
    signature.params() == &[ValueType::I64] && signature.return_type() == Some(ValueType::I32)
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
            ABI_TYPE_FORMER_RESOLVE_HANDLE_INDEX => {
                let handle = args.nth::<u64>(0) as usize;
                let result = args.nth::<u32>(1);

                let arity = match self.type_former_resolve_handle(&handle) {
                    None => return Ok(Some(KernelErrorCode::NoSuchTypeFormerRegistered.into())),
                    Some(arity) => arity.clone(),
                };

                self.write_memory(result, arity)?;

                Ok(Some(KernelErrorCode::Success.into()))
            }
            ABI_TYPE_FORMER_IS_HANDLE_REGISTERED_INDEX => unimplemented!(),
            ABI_TYPE_FORMER_REGISTER_HANDLE_INDEX => unimplemented!(),
            _otherwise => Err(host_trap(RuntimeTrap::NoSuchFunction)),
        }
    }
}

/// Maps an ABI host-call to its associated host-call number.  Also checks that
/// the function's signature is as expected, otherwise produces a runtime error
/// that is reported back to the WASM program.
impl ModuleImportResolver for RuntimeState {
    fn resolve_func(&self, field_name: &str, signature: &Signature) -> Result<FuncRef, WasmiError> {
        match field_name {
            ABI_TYPE_FORMER_RESOLVE_HANDLE_NAME => {
                if !check_type_former_resolve_handle(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TYPE_FORMER_RESOLVE_HANDLE_INDEX,
                ))
            }
            ABI_TYPE_FORMER_REGISTER_HANDLE_NAME => {
                if !check_type_former_register_handle(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TYPE_FORMER_REGISTER_HANDLE_INDEX,
                ))
            }
            ABI_TYPE_FORMER_IS_HANDLE_REGISTERED_NAME => {
                if !check_type_former_is_handle_registered(signature) {
                    return Err(host_error(KernelErrorCode::SignatureFailure));
                }

                Ok(FuncInstance::alloc_host(
                    signature.clone(),
                    ABI_TYPE_FORMER_IS_HANDLE_REGISTERED_INDEX,
                ))
            }
            otherwise => Err(host_error(KernelErrorCode::NoSuchFunction)),
        }
    }
}
