//! Token inputs less faithful to <https://www.usenix.org/system/files/sec21-salls.pdf> than [`EncodedInputs`]

use alloc::{string::String, vec::Vec};
use core::{
    fmt::Debug,
    hash::{BuildHasher, Hash, Hasher},
};

use ahash::RandomState;
use libafl_bolts::{prelude::OwnedSlice, rands::Rand, HasLen};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use super::{BytesInput, HasTargetBytes, Input, UsesInput};
use crate::{
    corpus::{CorpusId, Testcase},
    prelude::HasCorpus,
    stages::mutational::{MutatedTransform, MutatedTransformPost},
};

/// a Token
pub trait Token: Debug + Clone + Hash + Serialize + DeserializeOwned + PartialEq {
    /// the lexer for this Token Type
    type Lex: Lexer<Token = Self>;
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

/// a Lexer
pub trait Lexer: Sized {
    /// the tokens this lexer produces
    type Token: Token<Lex = Self>;
    /// lex the given source into Tokens
    /// ignoring any errors we encounter
    /// this should never panic
    fn lex(src: &[u8]) -> Vec<Self::Token>;
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
        self.tokens.extend_from_slice(other);
    }
}

impl<T: Token> Input for TokenInput<T> {
    #[must_use]
    fn generate_name(&self, _idx: usize) -> String {
        let mut hasher = RandomState::with_seeds(0, 0, 0, 0).build_hasher();
        for code in &self.tokens {
            hasher.write(code.as_bytes());
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
            bytes.extend_from_slice(token.as_bytes());
        }
        OwnedSlice::from(bytes)
    }
}

impl<S, T, Lex> MutatedTransform<BytesInput, S> for TokenInput<T>
where
    S: HasCorpus + UsesInput<Input = BytesInput>,
    T: Token<Lex = Lex>,
    Lex: Lexer<Token = T>,
{
    type Post = Self;

    fn try_transform_from(
        base: &mut Testcase<BytesInput>,
        state: &S,
        _corpus_idx: CorpusId,
    ) -> Result<Self, libafl_bolts::Error> {
        let input = base.load_input(state.corpus())?;
        Ok(TokenInput::new(T::Lex::lex(&input.bytes)))
    }

    fn try_transform_into(
        self,
        _state: &S,
    ) -> Result<(BytesInput, Self::Post), libafl_bolts::Error> {
        Ok((BytesInput::new(self.target_bytes().into()), self))
    }
}

impl<S, T> MutatedTransformPost<S> for TokenInput<T> where S: HasCorpus {}
