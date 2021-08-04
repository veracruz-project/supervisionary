//! # Tests for the Supervisionary type-former ABI
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

use libsupervisionary::type_former::{
    type_former_is_registered, type_former_resolve,
    PREALLOCATED_HANDLE_TYPE_FORMER_ARROW,
    PREALLOCATED_HANDLE_TYPE_FORMER_PROP,
};

fn main() {
    assert!(type_former_is_registered(
        PREALLOCATED_HANDLE_TYPE_FORMER_PROP
    ));

    assert!(type_former_is_registered(
        PREALLOCATED_HANDLE_TYPE_FORMER_ARROW
    ));

    assert_eq!(
        type_former_resolve(PREALLOCATED_HANDLE_TYPE_FORMER_PROP),
        Ok(0usize)
    );

    assert_eq!(
        type_former_resolve(PREALLOCATED_HANDLE_TYPE_FORMER_ARROW),
        Ok(2usize)
    );
}
