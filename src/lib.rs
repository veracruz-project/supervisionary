//! # Supervisionary: a privileged proof checker
//!
//! *Supervisionary* is a proof-checker for Classical Higher-order Logic (HOL)
//! in a similar vein to [Isabelle/HOL], [HOL4], [HOL Light], and similar
//! systems.  Supervisionary is structured in a broadly similar sense to the
//! aforementioned systems, in the sense that it provides a trusted,
//! distinguished *kernel* which is the only system component capable of
//! authoritatively authenticating or rejecting all claimed theorems in the
//! implemented logic.
//!
//! ## LCF: the *status quo*
//!
//! Existing implementations of HOL (and other logics) are organised along lines
//! pioneered by [Robin Milner] in the late-1970s.  This is the so-called "LCF
//! approach" named after the [LCF] proof-checking system that Milner was
//! designing when he and his team invented it, wherein the system's
//! proof-checking kernel is implemented as a module in a *meta-language* which
//! exposes an abstract type of theorems, with functions for constructing basic
//! theorems, corresponding to the axioms of the logic, and other functions
//! implementing the inference rules of the logic mapping existing theorems to
//! new theorems.  The soundness of the system, therefore depends crucially on
//! the soundness of the type- and module-system of the meta-language.  As a
//! result, most such systems are today written in strongly-typed functional
//! programming languages, such as [OCaml], [Standard ML], or [Haskell], all of
//! which descended from Milner's original meta-language, [ML], which was designed
//! explicitly for this task.
//!
//! Whilst this approach to system design obviously works well, and is widely
//! used, it does suffer from a number of disadvantages.
//!
//! First, the system's meta-language is privileged amongst all programming
//! languages.  This is because the system kernel, the component through which
//! all logical inference must be routed, is little other than a module written
//! in that meta-language.  As a result, all components wishing to communicate
//! with the system's kernel must also be written in that programming language,
//! or use a shim-layer written in that language to facilitate the interaction.
//! Whilst this need not necessarily be a problem, what *is* a problem is the
//! fact that proof-checking systems tend to be written in relatively niche
//! programming languages, with relatively small communities of users, like the
//! aforementioned Standard ML, OCaml, and Haskell.  Standard ML in particular
//! is a cause for concern, as it is to all intents and purposes a programming
//! language on "life support", barely kept alive for use in the Isabelle and
//! HOL4 systems.  Indeed, Standard ML has a single maintained compiler
//! implementation in [PolyML], kept alive almost exclusively by David Matthews.
//!
//! Second, we turn to the LCF approach itself.  Earlier, we mentioned that the
//! LCF approach was introduced to ensure system soundness, i.e. it was
//! essentially a way of ensuring the reliability of the system: when the
//! proof-checking kernel authenticates a theorem as being a legitimate theorem
//! of Classical HOL (or whatever other language your system is implementing)
//! then that should really be the case.  We can reinterpret LCF, and its design
//! goals, through an analysis typical of those found in the field of
//! *information security*, asking what it is that the LCF implementation style
//! protects, and under what assumptions that protection is effective.
//!
//! LCF, under this interpretation, is intended to maintain some important
//! system invariants in the face of a potential "attacker" who is trying to
//! subvert those invariants, in this case produce an element of the kernel's
//! theorem type which is *not* a legitimate theorem of the implemented logic.
//! Following this, we then are entitled to ask: "against which class of
//! attacker is the LCF approach effective", that is, what is the *threat model*
//! associated with the LCF approach?  Unfortunately, LCF does not protect
//! against much, only protecting against a class of attacker willing to limit
//! themselves to "attacks" composable in the meta-language used to implement
//! the system.  Once an attacker is willing to step outside this
//! straight-jacket, and step outside of this threat model, the LCF approach
//! crumbles.  LCF, at best, helps maintain system soundness in the face of
//! relatively benign attackers, essentially protecting only against programming
//! mistakes and implementation bugs.
//!
//! For current systems, this threat model is more than sufficient: theorem
//! proving remains a generally solitary endeavour, usually supplemented with a
//! small team of trusted collaborators.  However, there are imaginable changes
//! to the practice of theorem proving where a strengthened threat model might
//! be useful, for example in some models of distributed theorem proving.
//!
//! ## An alternative
//!
//! Is there an alternative to the LCF approach?  One seems to be staring us in
//! the face, as there is another type of software built around a trusted
//! *kernel* tasked with maintaining important system invariants, even in the
//! face of malicious or buggy clients, namely an *operating system*.
//!
//! Operating systems are just one example of a class of related software
//! systems, including [hypervisors], security monitors such as [Arm Trusted
//! Firmware-A], and (process) [virtual machines] (in the [JVM] sense of the
//! word).  What each of these software systems have in common is that they exist
//! at a privileged level compared to client code interacting with them ---
//! userspace code in the case of operating systems, virtual machines in the
//! case of hypervisors, and executing programs in the case of virtual machines.
//! Each provide services to client code, invokable via a binary interface
//! usable from any programming language capable of producing code conforming to
//! that interface, and are protected from their untrusted clients either by
//! hardware enforcement mechanisms like processor privilege levels, or in the
//! case of virtual machines through software equivalents.
//!
//! What, then, would a proof-checker look like if it was organized in a similar
//! manner to supervisory software, as discussed above?
//!
//! First, and most obviously, the proof-checking kernel would no longer be
//! implemented as a library in a meta-language, but as privileged code
//! separated from its client.  Communication between the kernel and client code
//! then is no longer facilitated by a module's API, but rather through an *ABI*
//! ([application binary interface]) shared between the kernel and client code.
//! Then, client code need not be written in the same language as the kernel,
//! moreover, client code need not be written in any one language at all, but
//! rather can be written in multiple languages, linked together, just as most
//! applications running under, e.g. Linux or Windows, are typically the product
//! of multiple libraries written in different programming languages.
//!
//! Second, the interface between kernel and client code needs to change
//! substantially.  The key device in the LCF approach is an abstract type of
//! theorems, which is no longer tenable when designing an ABI.  Rather, the
//! kernel needs to maintain a set of internal datastructures, in memory
//! inaccessible to client code, corresponding to constructed terms, types, and
//! theorems and other kind of *kernel object*.  Whilst clients may direct the
//! construction and manipulation of these objects, they do so indirectly, by
//! requesting that the kernel do so on their behalf.  Individual kernel objects
//! are referenced by client code through opaque handles, with term, type, and
//! theorem creation functionality exposed by the kernel returning handles to
//! client code that they can record in book-keeping datastructures as
//! appropriate.  Following this approach, system soundness is not inherited
//! from the soundness of the meta-language's type-system, but rather is
//! guaranteed by hardware security mechanisms.
//!
//! Third, the structure of client code must change.  Many operating systems
//! ship with a `libc`, an implementation of the C standard library, which
//! abstracts over the basic syscall binary interface and provides more
//! convenient functionality for other code to build on.  Importantly, this
//! `libc` implementation is untrusted from the perspective of the kernel, but
//! implicitly trusted by userspace software which builds on top of it.  In
//! analogy with this, any proof-checker organized along the lines of an
//! operating system needs a `libc` equivalent, able to abstract over the basic
//! binary interface exposed by the operating system.  Moreover, functionality
//! traditionally kept outside of the kernel in a system like Isabelle and HOL4,
//! such as pretty-printing, proof-state management for backwards proof,
//! automation, and so on, will also be provided by this trusted library that
//! exists outside of the kernel, in *prover-space*.  Importantly, whilst this
//! separation between kernel- and extra-kernel code exists in proof checking
//! systems today, the two are typically bound in a single monolithic codebase.
//! By organizing a proof-checking system along typical operating system lines,
//! it becomes possible to completely *decouble* the two completely, and even
//! swap out different syscall abstraction layers completely, just as one is
//! able to link against different implementations of `libc`, today.
//!
//! ## Supervisionary
//!
//! *Supervisionary* is a proof-checker implemented as supervisory software, as
//! described above, separated and protected from software executing in
//! prover-space.
//!
//! Rather than implement Supervisionary as a true operating system, we
//! implement Supervisionary as a virtual machine runtime.  We use [WebAssembly],
//! or WASM henceforth, as our executable format with the Supervisory therefore
//! acting as a host offering (proof-checking) services to prover-space
//! software through a series of host functions, akin to syscalls in an
//! operating system.  We build on the [WASMI] interpreter for WASM, as a means
//! of quickly building a prototype.  An implementation for other WASM execution
//! engines is of course possible.  Note that implementing Supervisionary as a
//! WASM host allows us to capture the essential essence of the design proposal,
//! without becoming inundated with trivia typically associated with writing an
//! operating system (e.g. booting a device).
//!
//! In particular, this crate contains the Supervionary kernel implementation.
//! Other crates also contain implementations of a `libc` analogue, as described
//! above, called `libsuper`.
//!
//! ## Supervisionary as an experiment in operating system design
//!
//! Above, we motivated Supervisionary as an experiment in proof-checker design,
//! essentially pitching Supervisionary's split between kernel-space and
//! prover-space, mediated by a defined ABI, in opposition to the prevailing LCF
//! design as implemented by most major proof assistants today.
//!
//! However, there is another interpretation of Supervisionary, essentially
//! treating the system as an experiment in operating system design instead,
//! exploring what happens if proof-checking is offered as a first-class service
//! in an operating system context, just as cryptographic and file-management
//! services have long been offered to user-space software by most operating
//! systems.
//!
//! To understand why this might be interesting consider the fact that, as
//! Supervisionary is merely a WASM host, any C or Rust program that does not
//! make use of any typical system service (e.g. the file-system, devices, IO,
//! and so on) can be compiled to WASM and then executed "under" Supervisionary.
//! Naturally, the class of programs that makes no use of any system service is
//! rather small, so it's interesting to wonder what might happen if we extend
//! the Supervisionary host ABI further, to offer more than proof-checking
//! functionality.  Let's assume, for example, that we choose to extend
//! Supervisionary with file-system functionality, allowing the program
//! executing under Supervisionary to open, read from, and write to files.
//! Assuming this, we then have two distinct type of service offered by
//! Supervisionary that remain completely separate:
//!
//! 1. Proof-checking services, as discussed in the sections above, and
//! 2. File-system services, as discussed here.
//!
//! What happens, then, if we assume that these two sets of services are no
//! longer kept completely separate, but for example file-system access
//! functionality was extended, so that the ABI host-call dedicated to opening a
//! file took not only a filename and mode as parameters, as one might typically
//! expect, but also took a handle to a *theorem*.  Essentially, this theorem
//! handle represents a *challenge* from the kernel to the caller of the host
//! call that must be discharged by the caller to avoid an error code being
//! returned.
//!
//! Before moving onto more complex examples of challenges, let's focus on two
//! very simple challenges that Supervisionary may pose to prover-space code as
//! they try to open a file: `true` and `false`.  In most logics, including HOL,
//! `true` is trivially easy to prove, and therefore represents no challenge at
//! all to whoever invokes the host-call.  On the other hand, in most logics
//! (and HOL in particular) `false` is impossibly hard to prove without assuming
//! additional axioms.  As a result, a challenge of `false` from Supervisionary
//! to all intents and purposes "seals off" the host-call from being called by
//! software from prover-space.  The two challenges, `true` and `false`,
//! therefore represent extremal points on a range of possible challenges, from
//! the impossibly easy to the impossibly hard.
//!
//! In particular, note that Supervisionary as supervisory software is capable
//! of *inspecting* the runtime state of the prover-space program, and recording
//! previous interactions that the prover-space software had with it, at the
//! point of host-call invocation: the program's heap, its trace of
//! previous host-calls and their arguments, and so on and so forth, for
//! example.  Moreover, when constructing a challenge to prover-space software,
//! Supervisionary is entitled to make use of all of this information when
//! constructing the challenge.  In short, the challenges that Supervisionary
//! can issue to prover-space code can be *functions* of the current state, and
//! these challenges can therefore be seen as *restrictions* on the machine
//! states from which a host-call can be made without causing a runtime error.
//! In this light, the two extremal challenges, `true` and `false`, can be
//! reinterpreted: a `true` challenge allows a host-call to be invoked in *any*
//! machine state, whereas a `false` challenge allows a host-call to be invoked
//! in *no* machine state.
//!
//!
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
//! [application binary interface]: https://en.wikipedia.org/wiki/Application_binary_interface
//! [Arm Research]: https://www.arm.com/resources/research
//! [Arm Trusted Firmware-A]: https://developer.arm.com/tools-and-software/open-source-software/firmware/trusted-firmware
//! [Dominic Mulligan]: https://dominic-mulligan.co.uk
//! [Haskell]: https://www.haskell.org
//! [HOL4]: https://hol-theorem-prover.org
//! [HOL Light]: https://www.cl.cam.ac.uk/~jrh13/hol-light/
//! [hypervisors]: https://en.wikipedia.org/wiki/Hypervisor
//! [Isabelle/HOL]: https://isabelle.in.tum.de
//! [JVM]: https://en.wikipedia.org/wiki/Java_virtual_machine
//! [LCF]: https://en.wikipedia.org/wiki/Logic_for_Computable_Functions
//! [ML]: https://homepages.inf.ed.ac.uk/wadler/papers/papers-we-love/milner-type-polymorphism.pdf
//! [OCaml]: https://ocaml.org
//! [PolyML]: https://www.polyml.org
//! [Robin Milner]: https://en.wikipedia.org/wiki/Robin_Milner
//! [Standard ML]: https://en.wikipedia.org/wiki/Standard_ML
//! [virtual machines]: https://en.wikipedia.org/wiki/Virtual_machine#Process_virtual_machines
//! [WASMI]: https://paritytech.github.io/wasmi/wasmi/index.html
//! [WebAssembly]: https://webassembly.org

/// The trusted kernel module.
pub mod kernel;
