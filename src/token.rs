use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::hash::Hash;

use crate::program::InstrPtr;

/// A single token for matching
pub trait Token: Eq + Debug {
    type Map: TokenMap<Self>;
    type Set: TokenSet<Self>;

    /// Returns whether the `Token` should be considered a word character.
    fn is_word(&self) -> bool;
}

impl Token for char {
    type Map = HashMap<char, InstrPtr>;
    type Set = HashSet<char>;

    /// Returns `false` if the character is whitespace, `true` otherwise.
    fn is_word(&self) -> bool {
        !self.is_whitespace()
    }
}

pub trait TokenMap<T: ?Sized> {
    fn get(&self, tok: &T) -> Option<InstrPtr>;
}

impl<T: Eq + Hash> TokenMap<T> for HashMap<T, InstrPtr> {
    fn get(&self, tok: &T) -> Option<InstrPtr> {
        self.get(tok).cloned()
    }
}

pub trait TokenSet<T: ?Sized> {
    fn contains(&self, tok: &T) -> bool;
}

impl<T: Eq + Hash> TokenSet<T> for HashSet<T> {
    fn contains(&self, tok: &T) -> bool {
        self.contains(tok)
    }
}
