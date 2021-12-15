# WASMI bindings

This module binds the Supervisionary kernel to the WASMI execution engine for Wasm (note that WASMI is an interpreter, rather than a JIT, for Wasm).
System calls made by the untrusted Wasm binary are routed, by this module, to their implementations in the execution engine-agnostic kernel.
