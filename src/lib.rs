//! Welcome to the Basis Core. While we realize there are many choices when it
//! comes to rust-based economic libraries that facilitate a profitless
//! socialist mode of production, we like to believe this one is the most
//! complete and featureful.
//!
//! This library, as said above, provides a functional interface for interacting
//! with a graph of economic nodes engaging in a socialist mode of production.
//! What this means is that we start from the concepts that
//!
//! 1. People should be free to determine their own path in life (bottom-up
//! organization)
//! 1. Companies started within this network operate without profit
//! 1. Economic planning is built-in but not required
//!
//! Effectively, this is a codebase designed to support [the free association
//! of producers][freeassoc],
//! a system of production sought after by Marxists and Anarchists in which
//! people are free to engage in production without the shackles of currency,
//! profits, or top-down planning structures.
//!
//! While this ideal is a long ways away, it is nonetheless worth striving for.
//! We also recognize that there will be inevitable transitional periods between
//! our current capitalist system and, *ahem*, better arrangements, so this
//! library also contains methods for interacting with capitalist markets in a
//! way that does not require compromising the ideals of the member companies.
//! For more information on the Basis project, see [the project website][basis].
//!
//! This library does not deal with storage or other external mediums in any way
//! and is fully self-contained. All data being operated on needs to be passed
//! in, and the results of the computations are returned and must be stored in a
//! place of your choosing. This allows Basis to exist beyond one particular
//! implementation of whatever storage medium du jour and exist beyond one
//! particular database structure.
//!
//! To get started, you will want to look at the [transactions]. Transactions
//! are the main interface for interacting with Basis.
//!
//! [freeassoc]: https://en.wikipedia.org/wiki/Free_association_(Marxism_and_anarchism)
//! [basis]: https://basisproject.gitlab.io/public/
//! [transactions]: transactions/

pub mod error;
mod util;
#[macro_use]
pub mod access;
pub mod models;
pub mod costs;
pub mod transactions;

