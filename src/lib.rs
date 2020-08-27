//! Welcome to the Basis Core. We realize there are many choices when evaluating
//! rust-based economic libraries that facilitate a socialist mode of production
//! and appreciate your choice of using Basis to further your revolutionary
//! goals.
//!
//! This library provides a functional interface for interacting with a graph of
//! economic nodes engaging in a socialist mode of production. What this means
//! is that we start from the concepts that
//!
//! 1. People should be free to determine and fulfill their own needs (bottom-up
//! organization)
//! 1. Companies within this network operate without profit
//! 1. Productive instruments and property are shared and managed by members
//!
//! Effectively, this is a codebase designed to support [the free association
//! of producers][freeassoc], a system of production sought after by Marxists
//! and Anarchists in which people are free to engage in production without
//! coercion.
//!
//! While this ideal is a long ways away, it is nonetheless worth striving for.
//! We also recognize that there will be inevitable transitional periods between
//! our current capitalist system and better arrangements, so this library also
//! contains methods for interacting with capitalist markets in a way that does
//! not require compromising the ideals of the member companies. For more
//! information on the Basis project, see [the project website][basis].
//!
//! This library does not deal with storage or other external mediums in any way
//! and is fully self-contained. All data being operated on needs to be passed
//! in, and the results of the computations are returned and must be stored in a
//! place of your choosing. This allows Basis to transcend any particular
//! storage medium and exist as a self-contained kernel that can be implemented
//! anywhere its data model is supported.
//!
//! To get started, you will want to look at the [transactions]. Transactions
//! are the main interface for interacting with Basis.
//!
//! [freeassoc]: https://en.wikipedia.org/wiki/Free_association_(Marxism_and_anarchism)
//! [basis]: https://basisproject.net/
//! [transactions]: transactions/

pub mod error;
mod util;
#[macro_use]
pub mod access;
pub mod models;
pub mod costs;
pub mod transactions;

