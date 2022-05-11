# The Supervisionary proof-checking system

## About

Supervisionary is an experimental proof-checking system for Gordon's higher-order logic ("HOL").
Rather than using programming language features to isolate and protect the kernel, as in typical LCF-style implementations of HOL like HOL4 and Isabelle, Supervisionary uses *privelege* akin to how an operating system kernel is isolated and protected from untrusted user-space code.

Ultimately, we hope to be able to use Veracruz to constrain the behaviour of untrusted programs in the Veracruz privacy-preserving distributed systems framework.
At this point, fully exploring this idea is still a work-in-progress.

## Citing Supervisionary

For more information, see the `paper/prisc22.tex` paper, or a pre-built [PDF](https://dominicpm.github.io/publications/mulligan-supervisionary-2022.pdf) of our accepted PriSC 2022 talk on Supervisionary.

To cite Supervisionary, for whatever reason, please use:

```
@techreport { mulligan-spinale-supervisonary-2022,
  title = {The {Supervisionary} proof-checking kernel (or: a work-in-progress towards proof-generating code)},
  authors = {Dominic P. Mulligan and Nick Spinale},
  institution = {Systems Research Group, Arm Research},
  type = {Extended abstract},
  note = {Presented at {PriSC 2022}},
  pages = {2},
  doi = {\url{https://doi.org/10.48550/arXiv.2205.03332}},
  year = {2022},
}
```
