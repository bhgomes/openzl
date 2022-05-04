# openzl

![OpenZL](https://user-images.githubusercontent.com/720571/166742171-cdf519f6-7af6-46c5-90b7-b24f15503c14.svg)

## Overview

openzl is an open-source library that helps practioners (especially in Web3 space) to develop and deploy secure, high performance zero-knowledge proof code in production. It tries to bridge the gap between low level cryptographic primitives and devlopers' need to build scalable protocols using zero-knowlege proof cryptography securely and quickly. More specifically, many developers today want to leverage zero-knowledge proof systems to build powerful protocols like ZCash/Manta/ZKSync. However, they are facing two less than ideal choices; first, building a protocol using high-level languages like [Circom](https://docs.circom.io) or [Cairo](https://www.cairo-lang.org) loses many performance optimization opportunities, and second, building the protocol directly using libraries like [arkworks/groth16](https://github.com/arkworks-rs/groth16), [zk-garage/plonk](https://github.com/zk-garage/plonk), or [microsoft/nova](https://github.com/microsoft/Nova) requires expertise in cryptography and can be very error-prone. Also, zero-knowledge proof systems are a moving target. There have been many new, and "better" proof systems coming out every 2-3 years ([BCTV](https://eprint.iacr.org/2013/879.pdf) -> [Groth16](https://eprint.iacr.org/2016/260.pdf) -> [Plonk](https://eprint.iacr.org/2019/953) -> [Nova](https://eprint.iacr.org/2021/370)). openzl tries to solve this problem by building flexible, proof-system agnostic, and extensible libraries for Web3 practioners.  

openzl consists of 3 parts:
* *Gadget libraries*: a library of gadgets that developers can uses as building blocks of their protocol. The initial range of the gadgets include accumulator (merkle tree with zero-knowledge membership proof), zk-friendly hashing (poseidon), and commitment schemes. The gadget libraries are programmed in `eclair`.
* *Embedded Circuit Language and Intermediate Representation (eclair)*: An embedded DSL in Rust that describe the circuit logic. eclair leverages Rust's expressive type systems to rule out certain class of errors during the circuit constructions such as (TODO).
* *Adaptors to Proof Systems*: Adaptors that convert circuit logic in eclair to the constraint systems used in different proof system backend, the initial supported proof systems are arkworks/groth16, zk-garage/plonk, and microsoft/nova.

## Openzl Goals and Non-Goals

### Goals
* A *production ready* and *proof-system agnostic* ZK library for blockchain applications (support [arkworks/groth16](https://github.com/arkworks-rs/groth16), [zk-garage/plonk](https://github.com/zk-garage/plonk), [microsoft/nova](https://github.com/microsoft/Nova)).
* `eclair` (**E**mbedded **C**ircuit **L**anguage **A**nd **I**ntermediate **R**epresentation): A shallow embedded circuit DSL in Rust that can rule out some common errors using Rust's type systems and still allow for optimizing circuits.
* Common gadgets such as hash functions, commitment schemes, accumulators, and more in `eclair`.
* Able to compile both prover and verifier to standard WASM and substrate flavored WASMI.
* Tutorials to support substrate ecosystem zero-knowledge proof applications.

### Non-Goals

* Build high-level languages like Circom and Cairo (would love to see someone else build high-level languages that compile to `eclair` though).
* Build "yet another PLONK". 
* Create more fragmentation in ZK tooling space.

## Openzl Design By Parts
### Gadget Library
Openzl provides list of cryptographic primitives with *optimized* zero-knowledge proof implementation in eclair. 
These gadgets are composable and can be combined to build more powerful protocols such as anonymous payment (ZCash/Manta) or zkrollups. The gadget library that openzl provides on its initial release includes:
* *hashing gadget*: an optimized implementation of poseidon hash[1], with parameterized arity (2, 4, 8)
* *accumulator gadget*: merkle tree gadget that supports using zero-knowlegde membership proof. The merkle tree gadget support the incremental updates as well.
* *commitment gadget*: A commitment scheme that is both *binding* and *hiding*, this commitment scheme is build on top of the *hashing gadget*.

### Embedded Circuit Language and Intermediate Representation (eclair)

Embedded Circuit Language and Intermediate Representation (eclair) is a shallow embedded DSL within Rust that serves the circuit description language in openzl stack. It has the following design considerations:
* *Proof system agnostic*: `eclair` is an IR that describe the circuit logic instead of lower level proof systems specific semantics and optimizations.
* *Unifying native and constraint code*: Writing zero-knowledge proof code in common framework like `arkworks`, it requires the programmers writing the same logic twice, one for constraints generation, one for native execution. This create a huge burden on developers also error prone. `eclair` solves this problem elegantly (see later example) by introducing the concept of "compiler". Openzl developers only need to write the circuit logic in `eclair` once, and it compiles to both native code and constraints. Openzl developers not only write circuit logic one, also don't have to worry the disparity between the native code and constraint generating code (which is certainly a bug). In addition, openzl automatically generates sanity check code for both native execution and constraints generation.
* *ruling out common errors*: At *compile time*, openzl's eclair compiler checks that private witness stays private and the public input stays public. For example, if a circuit implementers misuse the private witness allocation, this could cause a leakage of sercret key in the protocol implementation.   

Below is an example of circuit logic in `eclair` (this is Manta testnet V2 code in [manta-rs](https://github.com/Manta-Network/manta-rs) ):
```rust
impl<C> SenderVar<C>
where
    C: Configuration,
{
    /// Returns the asset for `self`, checking if `self` is well-formed in the given `compiler`.
    #[inline]
    pub fn get_well_formed_asset(
        self,
        parameters: &FullParametersVar<C>,
        compiler: &mut C::Compiler,
    ) -> AssetVar<C> {
        let public_spend_key = parameters.derive(&self.secret_spend_key, compiler);
        let utxo = parameters.utxo(
            &self.ephemeral_secret_key,
            &public_spend_key,
            &self.asset,
            compiler,
        );
        self.utxo_membership_proof.assert_valid(
            &parameters.utxo_accumulator_model,
            &utxo,
            compiler,
        );
        let void_number = parameters.void_number(&self.secret_spend_key, &utxo, compiler);
        compiler.assert_eq(&self.void_number, &void_number);
        self.asset
    }
}
```

The above code snippet describes the logic of checking well-formness of the private input in private transfer. Line 57 deverives the public key from the secret key. Then, Line 58 generates the UTXO from the `ephemeral_secret_key`, `public_spend_key`, and the asset. Line 64 checks the membership proof of the UTXO is valid. Line 69 generates the `void_number` from the secret key and the UTXO. Finally, line 70 checks the computed void number is the same as we passed in as a public input. 

One observation here is that the code passed in `Compiler` as an argument. This is due to the extensive design of `openzl`. When the `Compiler` argument is `native`, or simply passing in `()` (since `native` is the default compiler), this piece of `eclair` code will do the native execution. When the `Compiler` arguemtn is `Groth16`, this piece of code generates `R1CS` constraints for Groth16 proof system. When the `Compiler` argument is `Plonk`, this piece of code generates constraints in `Plonk` customized gates representations.

### Adaptors to Proof Systems
openzl implements adapters to the different constraints systems used in different underlying proof systems. The current supported underlying constaints systems include:
* R1CS in [arkworks/groth16](https://github.com/arkworks-rs/groth16) (support level: Production)
* Plonk constraints in [zk-garage/plonk](https://github.com/zk-garage/plonk) (support level: experimental)
* Relaxed R1CS in [microsoft/nova](https://github.com/microsoft/Nova) (support level: experimental)

These adapters compile `eclair` to the constraints in different constraints systems. This architecture is inspired by the modern compiler frameworks such as [LLVM](https://github.com/llvm/llvm-project).  In addition to merely being adapters, many proof system specific optimizations can be implemented in adapter level. For example, constraint reducing techniques to leverage customized gates in Plonk to reduce the constraints size in Plonk.

## Existing Prototype

Openzl is not building out of thin air. The majority of the code is live in Manta's internal infrastructure now ([manta-rs](https://github.com/Manta-Network/manta-rs). Below is the list of existing or migratable features:

| feature  | code |  audit | 
|----------|------| -------|
| gadget/hashing |  Complete | Not start | 
| gadget/commitment |  In progress  |  Not start |
| gadget/accumulator |  Compelete  |  Not start |
| eclair | Protype |  Not start |
| adapter/Groth16 | Complete | Not start |
| adapter/Plonk | In progress | Not start |
| adapter/Nova | Not start | Not start |

## Tutorials 

We will provide substrate specific tutorials to show case how to code an end to end example using openzl. Potential examples includes:
* Build a tornado.cash styled private IOU systems.
* Build a simple zk-rollup in for substrate-based payment.


## Openzl Milestones and Deliveries

* Milestone 1 (Prototype): July, 2022
   * Code complete for all gadget libraries
   * Code complete for `eclair`
   * Code complete for `Groth16` adapter
   * End to end example and test using `Groth16` backend
* Milestone 2 (Feature Complete): Sep, 2022
   * Code complete (experimental) for `plonk` backend
   * Code complete (experiemental) for `nova` backend
   * Spec complete for security audit 
* Milestone 3 (Audit): Nov. 2022
  Potential auditors: ABDK, Least Authority, Trail of Bits
  * Audit complete for all production level support code
  * Substrate based tutorials
  * Trusted-setup toolings 

## Project Budgets:
| Item  |  Budgets (USD) |  Remark  |
|-------|----------|---------|
| Developer salary  | 360,000  | (4 cryptographic engineer * 6 months + 1 devop engineer * 0.5 month )  |
| Audit   | 600,000 |  `40,000` LOC (currently, manta-rs has about 30,000 LOC) * `15 USD/LOC` (quote from ABDK) |
| CI/CD      |  4,000  |  CI/CD for openzl |
| Misc.   | 1,000 | DNS, Website Hosting, ect |

Totol budget: 1,001,000 USD (will converted to DOT using the exchange rate on application)

## Openzl team

### Oversight Committee
Oversight commitee will manage the overall execution and the financil budget of openzl,
* **Shumo Chu** (Co-founder, Manta Network)
* **Luke Pearson** (Research Partner, Polychain Capital)
* **Bryan Chen** (CTO, Acala Network)

Funding and spendings will be managed in a 2/3 multisig.

### Developing Team (Alphabetical) 
* **Boyuan Feng**: Cryptogrpahic Engineer at Manta, PhD Computer Science from UCSB, extensive zero-knowledge proof compiler experiences (e.g. first author of [ZEN](https://eprint.iacr.org/2021/087)).
* **Brandon Gome**: Cryptographic Engineer at Manta, BS Math from Rutgers, main author of [manta-rs](https://github.com/Manta-Network/manta-rs).
* **Todd Norton**: Cryptographic Engineer at Manta, PhD Physics from Caltech.
* **Tom Shen**: Cryptographic Engineer at Manta, BS Computer Science from UC Berkeley, [arkworks](https://github.com/arkworks-rs) core contributor. 
* **Rob Thijssen**: Devop Engineer at Manta, ex-Mozilla engineer.

## References
1. [poseidon hash](https://eprint.iacr.org/2019/458.pdf)

