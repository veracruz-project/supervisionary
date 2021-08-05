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
use libsupervisionary::type_former::PREALLOCATED_HANDLE_TYPE_FORMER_PROP;

fn main() {
    assert!(type_is_registered(PREALLOCATED_HANDLE_TYPE_ALPHA));
    assert!(type_is_registered(PREALLOCATED_HANDLE_TYPE_BETA));
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
    assert!(type_is_registered(PREALLOCATED_HANDLE_TYPE_UNARY_PREDICATE));

    assert_eq!(type_test_variable(PREALLOCATED_HANDLE_TYPE_ALPHA), Ok(true));
    assert_eq!(type_test_variable(PREALLOCATED_HANDLE_TYPE_BETA), Ok(true));

    assert_eq!(
        type_test_function(PREALLOCATED_HANDLE_TYPE_BINARY_CONNECTIVE),
        Ok(true)
    );
    assert_eq!(
        type_test_function(PREALLOCATED_HANDLE_TYPE_BINARY_PREDICATE),
        Ok(true)
    );
    assert_eq!(
        type_test_function(PREALLOCATED_HANDLE_TYPE_QUANTIFIER),
        Ok(true)
    );
    assert_eq!(
        type_test_function(PREALLOCATED_HANDLE_TYPE_UNARY_PREDICATE),
        Ok(true)
    );
    assert_eq!(
        type_test_function(PREALLOCATED_HANDLE_TYPE_UNARY_CONNECTIVE),
        Ok(true)
    );

    assert_eq!(
        type_test_combination(PREALLOCATED_HANDLE_TYPE_PROP),
        Ok(true)
    );
    assert_eq!(
        type_test_combination(PREALLOCATED_HANDLE_TYPE_BINARY_CONNECTIVE),
        Ok(true)
    );
    assert_eq!(
        type_test_combination(PREALLOCATED_HANDLE_TYPE_BINARY_PREDICATE),
        Ok(true)
    );
    assert_eq!(
        type_test_combination(PREALLOCATED_HANDLE_TYPE_QUANTIFIER),
        Ok(true)
    );
    assert_eq!(
        type_test_combination(PREALLOCATED_HANDLE_TYPE_UNARY_PREDICATE),
        Ok(true)
    );
    assert_eq!(
        type_test_combination(PREALLOCATED_HANDLE_TYPE_UNARY_CONNECTIVE),
        Ok(true)
    );

    assert_eq!(type_split_variable(PREALLOCATED_HANDLE_TYPE_ALPHA), Ok(0));
    assert_eq!(type_split_variable(PREALLOCATED_HANDLE_TYPE_BETA), Ok(1));

    assert_eq!(
        type_split_function(PREALLOCATED_HANDLE_TYPE_BINARY_CONNECTIVE),
        Ok((
            PREALLOCATED_HANDLE_TYPE_PROP,
            PREALLOCATED_HANDLE_TYPE_UNARY_CONNECTIVE
        ))
    );
    assert_eq!(
        type_split_function(PREALLOCATED_HANDLE_TYPE_BINARY_PREDICATE),
        Ok((
            PREALLOCATED_HANDLE_TYPE_ALPHA,
            PREALLOCATED_HANDLE_TYPE_UNARY_PREDICATE
        ))
    );
    assert_eq!(
        type_split_function(PREALLOCATED_HANDLE_TYPE_QUANTIFIER),
        Ok((
            PREALLOCATED_HANDLE_TYPE_UNARY_PREDICATE,
            PREALLOCATED_HANDLE_TYPE_PROP
        ))
    );
    assert_eq!(
        type_split_function(PREALLOCATED_HANDLE_TYPE_UNARY_PREDICATE),
        Ok((
            PREALLOCATED_HANDLE_TYPE_ALPHA,
            PREALLOCATED_HANDLE_TYPE_PROP
        ))
    );
    assert_eq!(
        type_split_function(PREALLOCATED_HANDLE_TYPE_UNARY_CONNECTIVE),
        Ok((PREALLOCATED_HANDLE_TYPE_PROP, PREALLOCATED_HANDLE_TYPE_PROP))
    );

    assert_eq!(
        type_split_combination(PREALLOCATED_HANDLE_TYPE_PROP),
        Ok((PREALLOCATED_HANDLE_TYPE_FORMER_PROP, Vec::new()))
    );
}
