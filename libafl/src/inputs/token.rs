//! Token inputs more faithful to https://www.usenix.org/system/files/sec21-salls.pdf than [`EncodedInputs`]

use alloc::{string::String, vec::Vec};
use core::{
    fmt::Debug,
    hash::{BuildHasher, Hash, Hasher},
};

use ahash::RandomState;
use libafl_bolts::{prelude::OwnedSlice, rands::Rand, HasLen};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use super::{HasTargetBytes, Input};

/// a Token
pub trait Token: Debug + Clone + Hash + Serialize + DeserializeOwned + PartialEq {
    /// create a new random token
    fn new_rand(rand: &mut impl Rand) -> Self;
    /// replace the token with a similar one
    /// one may for example replace a `var` with a `let` in js
    fn replace_rand_similar(&mut self, rand: &mut impl Rand) {
        *self = Self::new_rand(rand);
    }
    /// get the bytes representation of the given token
    fn as_bytes(&self) -> &[u8];
    /// get the corresponding closing/opening bracket
    /// the corresponding bracked for ';' in js for example would be ';'
    fn closing_bracket(&self) -> Option<&Self> {
        None
    }
}

/// a Token input
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TokenInput<T> {
    // TODO: we do lot's of splicings in the mutations
    // a LinkedList may be more efficient
    tokens: Vec<T>,
}

impl<T: Clone> TokenInput<T> {
    /// Creates a new codes input using the given tokens
    #[must_use]
    #[inline]
    pub fn new(tokens: Vec<T>) -> Self {
        Self { tokens }
    }
    /// The tokens of this tokne input
    #[must_use]
    #[inline]
    pub fn tokens(&self) -> &[T] {
        &self.tokens
    }

    /// The tokens of this token input, mutable
    #[must_use]
    #[inline]
    pub fn tokens_mut(&mut self) -> &mut [T] {
        &mut self.tokens
    }

    /// The remove one token from the input given an index
    #[inline]
    pub(crate) fn remove(&mut self, index: usize) -> T {
        self.tokens.remove(index)
    }

    #[inline]
    pub(crate) fn truncate(&mut self, len: usize) {
        self.tokens.truncate(len);
    }

    #[inline]
    pub(crate) fn extend_from_slice(&mut self, other: &[T]) {
        self.tokens.extend_from_slice(other)
    }
}

impl<T: Token> Input for TokenInput<T> {
    #[must_use]
    fn generate_name(&self, _idx: usize) -> String {
        let mut hasher = RandomState::with_seeds(0, 0, 0, 0).build_hasher();
        for code in &self.tokens {
            hasher.write(&code.as_bytes());
        }
        format!("{:016x}", hasher.finish())
    }
}
impl<T> HasLen for TokenInput<T> {
    #[inline]
    fn len(&self) -> usize {
        self.tokens.len()
    }
}

impl<T: Token> HasTargetBytes for TokenInput<T> {
    #[inline]
    fn target_bytes(&self) -> OwnedSlice<u8> {
        let mut bytes = vec![];
        for token in self.tokens() {
            bytes.extend_from_slice(token.as_bytes())
        }
        OwnedSlice::from(bytes)
    }
}
