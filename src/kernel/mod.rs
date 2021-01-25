//! # The Supervisionary kernel
//!
//! *Note that this is trusted code.*
//!
//! This module implements the Supervisionary kernel, introducing facilities for
//! building and manipulating *terms*, *types*, and *theorems*, and registering
//! new *type-formers* and *term* constants.  We refer to all of these objects,
//! collectively, as *kernel objects* as they are permanently kept under the
//! management of the kernel, never manipulated directly by prover-space
//! software.
//!
//! Note that terms and types are defined recursively in HOL.  For the ABI to
//! work effectively, we therefore need to break this recursion.  Instead, in
//! Supervisionary, terms and types are not directly recursive, but larger terms
//! are instead built from smaller terms by making reference to the *handle* of
//! previously constructed terms and types.  Terms and types must therefore be
//! registered, incrementally, with the kernel as they are built.  Various
//! useful terms, types, type-formers, and constants, are pre-registered in the
//! kernel, and the ABI defines fixed handles that prover-space code is expected
//! to know, by convention, to access these objects.  Interestingly, this
//! pattern of registering incremental terms and types allows a form of
//! *sharing*: once a term (for example) is constructed and registered with the
//! kernel, the handle that references it can be cached by prover-space code,
//! and reused whenever that term is needed again without reconstructing the
//! term from scratch.
//!
//! Note that, for soundness reasons, it's important that a term or type, once
//! registered with the kernel, remains *immutable*.  This is essential, as
//! otherwise handles, potentially cached in prover-space, would not remain
//! well-defined.
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

/// HOL types, operations upon them, and the kernel's registered type table.
pub mod _type;
/// Term constants.  These are registered with the kernel with a declared type.
/// The kernel maintains a constant table, mapping handles to types.
pub mod constant;
/// Error codes used to indicate errors across the ABI boundary.
pub mod error_code;
/// Handles used to uniquely identify kernel objects.  Various pre-allocated
/// handles are also defined in this module, used to refer to primitive kernel
/// objects.
pub mod handle;
/// HOL terms, and operations on them.  Formulae in HOL are identified with
/// terms with type `Prop`.
pub mod term;
/// Theorems, axioms, and inference rules of the logic.
pub mod theorem;
/// HOL type-formers.  These are registered with the kernel with a declared
/// arity.  The kernel maintains a type-former table, mapping handles to
/// arities.
pub mod type_former;
