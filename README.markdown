# The Supervisionary proof-checking system

Supervisionary is an experimental proof-checking system for Gordon's higher-order logic ("HOL").
Rather than using programming language features to isolate and protect the kernel, as in typical LCF-style implementations of HOL like HOL4 and Isabelle, Supervisionary uses *privelege* akin to how an operating system kernel is isolated and protected from untrusted user-space code.

Ultimately, we hope to be able to use Veracruz to constrain the behaviour of untrusted programs in the Veracruz privacy-preserving distributed systems framework.
At this point, fully exploring this idea is still a work-in-progress.

For more information, see the `paper/prisc22.tex` paper, or a pre-built [PDF](https://dominicpm.github.io/publications/mulligan-supervisionary-2022.pdf) of our accepted PriSC 2022 talk on Supervisionary.
