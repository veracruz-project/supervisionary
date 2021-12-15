# The Supervisionary kernel

This is the main implementation of the HOL proof-checking kernel.
As a result, this is **trusted code**.

Various datatypes are defined by the kernel, following the general pattern of how every HOL implementation is typically written:

- Type-formers, which have an associated arity,
- Polymorphic simple types, which are built out of variables and type-formers fully applied at their declared arity,
- Constants, which have a declared type,
- Terms of the lambda-calculus, which are built out of constants and explicitly-typed variables,
- Theorems, which are built out of a context of propositionally-typed terms and a single, propositionally-typed conclusion.

Note that this module is kept generic, as far as possible, in the Wasm engine that will be used to execute the untrusted program executing under Supervisionary's supervision.
