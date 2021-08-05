//! # Tests for the Supervisionary type ABI
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

use libsupervisionary::_type::*;

fn main() {
    assert!(type_is_registered(PREALLOCATED_HANDLE_TYPE_ALPHA));
    /* assert!(type_is_registered(PREALLOCATED_HANDLE_TYPE_BETA));
    assert!(type_is_registered(
        PREALLOCATED_HANDLE_TYPE_BINARY_CONNECTIVE
    ));
    assert!(type_is_registered(
        PREALLOCATED_HANDLE_TYPE_BINARY_PREDICATE
    ));
    assert!(type_is_registered(PREALLOCATED_HANDLE_TYPE_PROP));
    assert!(type_is_registered(PREALLOCATED_HANDLE_TYPE_QUANTIFIER));
    assert!(type_is_registered(
        PREALLOCATED_HANDLE_TYPE_UNARY_CONNECTIVE
    ));
    assert!(type_is_registered(PREALLOCATED_HANDLE_TYPE_UNARY_PREDICATE)); */
}
