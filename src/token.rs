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
    type Map = MapOrFn<char>;
    type Set = SetOrFn<char>;

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

pub enum MapOrFn<T> {
    Map(HashMap<T, InstrPtr>),
    Fn {
        func: Box<dyn Fn(&T) -> Option<InstrPtr>>,
        name: String,
    },
}

impl<T: Eq + Hash> TokenMap<T> for MapOrFn<T> {
    fn get(&self, tok: &T) -> Option<InstrPtr> {
        match self {
            MapOrFn::Map(m) => TokenMap::get(m, tok),
            MapOrFn::Fn { func, .. } => func(tok),
        }
    }
}

impl<T: Debug> Debug for MapOrFn<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            MapOrFn::Map(m) => m.fmt(f),
            MapOrFn::Fn { name, .. } => f.write_str(name),
        }
    }
}

#[cfg(test)]
impl<T: Eq + Hash> PartialEq for MapOrFn<T> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (MapOrFn::Map(s), MapOrFn::Map(o)) => s == o,
            (MapOrFn::Fn { name: s, .. }, MapOrFn::Fn { name: o, .. }) => s == o,
            _ => false,
        }
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

pub enum SetOrFn<T> {
    Set(HashSet<T>),
    Fn {
        func: Box<dyn Fn(&T) -> bool>,
        name: String,
    },
}

impl<T: Eq + Hash> TokenSet<T> for SetOrFn<T> {
    fn contains(&self, tok: &T) -> bool {
        match self {
            SetOrFn::Set(s) => TokenSet::contains(s, tok),
            SetOrFn::Fn { func, .. } => func(tok),
        }
    }
}

impl<T: Debug> Debug for SetOrFn<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            SetOrFn::Set(m) => m.fmt(f),
            SetOrFn::Fn { name, .. } => f.write_str(name),
        }
    }
}

#[cfg(test)]
impl<T: Eq + Hash> PartialEq for SetOrFn<T> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (SetOrFn::Set(s), SetOrFn::Set(o)) => s == o,
            (SetOrFn::Fn { name: s, .. }, SetOrFn::Fn { name: o, .. }) => s == o,
            _ => false,
        }
    }
}
