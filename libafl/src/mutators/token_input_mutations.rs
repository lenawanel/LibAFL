//! mutations similar to the ones in <https://www.usenix.org/system/files/sec21-salls.pdf>

use alloc::vec::Vec;
use core::ops::{Add, Range};

use libafl_bolts::{rands::Rand, HasLen, Named};

use super::{buffer_self_copy, MutationResult, Mutator};
use crate::{
    corpus::Corpus,
    inputs::{Token, TokenInput, UsesInput},
    prelude::{HasCorpus, HasMaxSize, HasRand},
    random_corpus_id,
};

macro_rules! trivial_mutator_impls {
    ($ty:ty) => {
        impl Named for $ty {
            fn name(&self) -> &str {
                stringify!($ty)
            }
        }
        impl $ty {
            #[doc = concat!("Creates a new [`", stringify!($ty), "`]")]
            #[must_use]
            pub fn new() -> Self {
                Self::default()
            }
        }
    };
}

/// replaces a Token from the input with a random Token
#[derive(Debug, Default)]
pub struct TokenRandMutator;

trivial_mutator_impls!(TokenRandMutator);

impl<S, T: Token> Mutator<S::Input, S> for TokenRandMutator
where
    S: UsesInput<Input = TokenInput<T>> + HasRand + HasCorpus,
{
    fn mutate(
        &mut self,
        state: &mut S,
        input: &mut TokenInput<T>,
        _stage_idx: i32,
    ) -> Result<MutationResult, libafl_bolts::Error> {
        if input.tokens().is_empty() {
            Ok(MutationResult::Skipped)
        } else {
            let val = state.rand_mut().choose(input.tokens_mut());
            *val = T::new_rand(state.rand_mut());
            Ok(MutationResult::Mutated)
        }
    }
}
/// delete a Token from the input
#[derive(Debug, Default)]
pub struct TokenDeleteMutator;

trivial_mutator_impls!(TokenDeleteMutator);

impl<S, T: Token> Mutator<S::Input, S> for TokenDeleteMutator
where
    S: UsesInput<Input = TokenInput<T>> + HasRand + HasCorpus,
{
    fn mutate(
        &mut self,
        state: &mut S,
        input: &mut TokenInput<T>,
        _stage_idx: i32,
    ) -> Result<MutationResult, libafl_bolts::Error> {
        if input.tokens().is_empty() {
            Ok(MutationResult::Skipped)
        } else {
            let idx = state.rand_mut().below(input.len().try_into().unwrap());
            input.remove(idx.try_into().unwrap());

            Ok(MutationResult::Mutated)
        }
    }
}

/// replaces a Token from the input with a similar one
/// the definition of similar here is what `<T as Token>::replace_rand_similar` considers similar
#[derive(Debug, Default)]
pub struct TokenRandSimilarMutator;

trivial_mutator_impls!(TokenRandSimilarMutator);

impl<S, T: Token> Mutator<S::Input, S> for TokenRandSimilarMutator
where
    S: UsesInput<Input = TokenInput<T>> + HasRand + HasCorpus,
{
    fn mutate(
        &mut self,
        state: &mut S,
        input: &mut TokenInput<T>,
        _stage_idx: i32,
    ) -> Result<MutationResult, libafl_bolts::Error> {
        if input.tokens().is_empty() {
            Ok(MutationResult::Skipped)
        } else {
            let val = state.rand_mut().choose(input.tokens_mut());
            val.replace_rand_similar(state.rand_mut());
            Ok(MutationResult::Mutated)
        }
    }
}

/// delete a Token from the input
#[derive(Debug, Default)]
pub struct TokenReplaceRegionSimilarMutator;

trivial_mutator_impls!(TokenReplaceRegionSimilarMutator);

impl<S, T: Token> Mutator<S::Input, S> for TokenReplaceRegionSimilarMutator
where
    S: UsesInput<Input = TokenInput<T>> + HasRand + HasCorpus,
{
    fn mutate(
        &mut self,
        state: &mut S,
        input: &mut TokenInput<T>,
        _stage_idx: i32,
    ) -> Result<MutationResult, libafl_bolts::Error> {
        let Some(region) = rand_region(state.rand_mut(), input.tokens()) else {
            return Ok(MutationResult::Skipped);
        };

        for tok in &mut input.tokens_mut()[region] {
            tok.replace_rand_similar(state.rand_mut());
        }

        Ok(MutationResult::Mutated)
    }
}

/// a helper function to search for closing bracket
#[inline]
fn locate_cl_br<T: Token>(cl_br: &T, buf: &[T]) -> Option<usize> {
    buf.iter().position(|tok| tok == cl_br)
}
/// delete a Token from the input
#[derive(Debug, Default)]
pub struct TokenReplaceRegionRandMutator;

trivial_mutator_impls!(TokenReplaceRegionRandMutator);

impl<S, T: Token> Mutator<S::Input, S> for TokenReplaceRegionRandMutator
where
    S: UsesInput<Input = TokenInput<T>> + HasRand + HasCorpus,
{
    fn mutate(
        &mut self,
        state: &mut S,
        input: &mut TokenInput<T>,
        _stage_idx: i32,
    ) -> Result<MutationResult, libafl_bolts::Error> {
        let Some(region) = rand_region(state.rand_mut(), input.tokens()) else {
            return Ok(MutationResult::Skipped);
        };

        for tok in &mut input.tokens_mut()[region] {
            *tok = T::new_rand(state.rand_mut());
        }

        Ok(MutationResult::Mutated)
    }
}

/// replace a random amount of subsqequent tokens at a time
#[derive(Debug, Default)]
pub struct TokenOverrideMutator;

trivial_mutator_impls!(TokenOverrideMutator);

impl<S, T: Token> Mutator<S::Input, S> for TokenOverrideMutator
where
    S: UsesInput<Input = TokenInput<T>> + HasRand + HasCorpus,
{
    fn mutate(
        &mut self,
        state: &mut S,
        input: &mut TokenInput<T>,
        _stage_idx: i32,
    ) -> Result<MutationResult, libafl_bolts::Error> {
        if input.tokens().is_empty() {
            Ok(MutationResult::Skipped)
        } else {
            let rand = state.rand_mut();
            let off1: usize = rand.next().try_into().unwrap();
            let off2: usize = rand.next().try_into().unwrap();
            let range = core::cmp::min(off1, off2)..core::cmp::max(off1, off2);

            for val in &mut input.tokens_mut()[range] {
                *val = T::new_rand(rand);
            }

            Ok(MutationResult::Mutated)
        }
    }
}

/// replace a random amount of subsqequent tokens at a time with similar ones
#[derive(Debug, Default)]
pub struct TokenSimilarOverrideMutator;

trivial_mutator_impls!(TokenSimilarOverrideMutator);

impl<S, T: Token> Mutator<S::Input, S> for TokenSimilarOverrideMutator
where
    S: UsesInput<Input = TokenInput<T>> + HasRand + HasCorpus,
{
    fn mutate(
        &mut self,
        state: &mut S,
        input: &mut TokenInput<T>,
        _stage_idx: i32,
    ) -> Result<MutationResult, libafl_bolts::Error> {
        if input.tokens().is_empty() {
            Ok(MutationResult::Skipped)
        } else {
            let rand = state.rand_mut();
            let len = input.len().try_into().unwrap();
            let off1: usize = rand.below(len).try_into().unwrap();
            let off2: usize = rand.below(len).try_into().unwrap();
            let range = core::cmp::min(off1, off2)..core::cmp::max(off1, off2);

            for val in &mut input.tokens_mut()[range] {
                val.replace_rand_similar(rand);
            }

            Ok(MutationResult::Mutated)
        }
    }
}

/// delete a random slice from the [`TokenInput`]
#[derive(Debug, Default)]
pub struct TokenDeleteManyMutator;

trivial_mutator_impls!(TokenDeleteManyMutator);

impl<S, T: Token> Mutator<S::Input, S> for TokenDeleteManyMutator
where
    S: UsesInput<Input = TokenInput<T>> + HasRand + HasCorpus,
{
    fn mutate(
        &mut self,
        state: &mut S,
        input: &mut TokenInput<T>,
        _stage_idx: i32,
    ) -> Result<MutationResult, libafl_bolts::Error> {
        let rand = state.rand_mut();
        let len = input.len().try_into().unwrap();
        let off1: usize = rand.below(len).try_into().unwrap();
        let off2: usize = rand.below(len).try_into().unwrap();
        let range = core::cmp::min(off1, off2)..core::cmp::max(off1, off2);

        unsafe {
            buffer_self_copy(
                input.tokens_mut(),
                range.end,
                range.start,
                len as usize - range.end,
            );
        }

        input.truncate(range.end);

        Ok(MutationResult::Mutated)
    }
}

/// delete a random region from the [`TokenInput`]
#[derive(Debug, Default)]
pub struct TokenDeleteRegionMutator;

trivial_mutator_impls!(TokenDeleteRegionMutator);

impl<S, T: Token> Mutator<S::Input, S> for TokenDeleteRegionMutator
where
    S: UsesInput<Input = TokenInput<T>> + HasRand + HasCorpus,
{
    fn mutate(
        &mut self,
        state: &mut S,
        input: &mut TokenInput<T>,
        _stage_idx: i32,
    ) -> Result<MutationResult, libafl_bolts::Error> {
        let rand = state.rand_mut();
        let len = input.len();

        let Some(range) = rand_region(rand, input.tokens()) else {
            return Ok(MutationResult::Skipped);
        };

        unsafe {
            buffer_self_copy(input.tokens_mut(), range.end, range.start, len - range.end);
        }

        input.truncate(range.end);

        Ok(MutationResult::Mutated)
    }
}

/// Insert a random slice of tokens into the [`TokenInput`]
#[derive(Debug, Default)]
pub struct TokenInsertMutator<T> {
    buf: Vec<T>,
}

impl<T> Named for TokenInsertMutator<T> {
    fn name(&self) -> &str {
        "TokenInsertMutator"
    }
}

impl<T> TokenInsertMutator<T> {
    #[doc = "Creates a new [`TokenInsertMutator`]"]
    #[must_use]
    pub fn new() -> Self {
        Self {
            buf: Vec::with_capacity(INSERT_TOKEN_MAX.try_into().unwrap()),
        }
    }
}

const INSERT_TOKEN_MAX: u64 = 64;

impl<S, T: Token> Mutator<S::Input, S> for TokenInsertMutator<T>
where
    S: UsesInput<Input = TokenInput<T>> + HasRand + HasCorpus,
{
    fn mutate(
        &mut self,
        state: &mut S,
        input: &mut TokenInput<T>,
        _stage_idx: i32,
    ) -> Result<MutationResult, libafl_bolts::Error> {
        let rand = state.rand_mut();
        let token_nr = rand.below(INSERT_TOKEN_MAX);
        self.buf
            .resize_with(token_nr.try_into().unwrap(), || T::new_rand(rand));

        let idx = rand.below(input.len().try_into().unwrap());

        self.buf
            .extend_from_slice(&input.tokens()[idx.try_into().unwrap()..]);

        // TODO: this may be faster using sth like `Vec::spare_capacity_mut`
        input.truncate(idx.try_into().unwrap());
        input.extend_from_slice(&self.buf);
        self.buf.truncate(0);

        Ok(MutationResult::Mutated)
    }
}

/// replace a region from one [`TokenInput`] with another
#[derive(Debug, Default)]
pub struct TokenSpliceRegionMutator<T> {
    buf: Vec<T>,
}

impl<T> Named for TokenSpliceRegionMutator<T> {
    fn name(&self) -> &str {
        "TokenSpliceRegionMutator"
    }
}
impl<T> TokenSpliceRegionMutator<T> {
    #[doc = "Creates a new [`TokenSpliceRegionMutator`]"]
    #[must_use]
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }
}

impl<S, T: Token> Mutator<S::Input, S> for TokenSpliceRegionMutator<T>
where
    S: UsesInput<Input = TokenInput<T>> + HasRand + HasCorpus + HasMaxSize,
    <S as HasRand>::Rand: Clone,
{
    fn mutate(
        &mut self,
        state: &mut S,
        input: &mut TokenInput<T>,
        _stage_idx: i32,
    ) -> Result<MutationResult, libafl_bolts::Error> {
        let idx = random_corpus_id!(state.corpus(), state.rand_mut());

        let rand = state.rand_mut();

        let Some(region) = rand_region(rand, input.tokens()) else {
            return Ok(MutationResult::Skipped);
        };

        {
            // this is a nasty solution to get around the borrow checker
            let mut rand = state.rand_mut().clone();
            let mut testcase = state.corpus().get(idx)?.borrow_mut();
            let other_input = testcase.load_input(state.corpus())?.tokens();

            let Some(region) = rand_region(&mut rand, other_input) else {
                return Ok(MutationResult::Skipped);
            };

            self.buf.extend_from_slice(&other_input[region]);
        };

        self.buf.extend_from_slice(&input.tokens()[region.end..]);

        input.truncate(region.start);
        input.extend_from_slice(&self.buf);

        self.buf.truncate(0);

        Ok(MutationResult::Mutated)
    }
}

fn rand_region<T: Clone + Token>(rand: &mut impl Rand, input: &[T]) -> Option<Range<usize>> {
    let op_brs = input
        .iter()
        .enumerate()
        .filter_map(|(idx, token)| <T as Token>::closing_bracket(token).map(|tok| (tok, idx)))
        // TODO: avoid allocating here
        .collect::<Vec<_>>();
    if op_brs.is_empty() {
        return None;
    }

    let (cl_br, op_br_idx) = rand.choose(op_brs);

    let cl_br_idx = locate_cl_br(cl_br, &input[op_br_idx..]).map(|idx| idx.add(op_br_idx));
    cl_br_idx.map(|cl_br_idx| op_br_idx..cl_br_idx)
}
