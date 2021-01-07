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

#[macro_export]
macro_rules! map {
    ($($key:expr => $value:expr),*$(,)?) => {
        {
            let _cap = $crate::__count!($($key),*);
            let mut _map = std::collections::HashMap::with_capacity(_cap);
            $(
                _map.insert($key, $value);
            )*
            _map
        }
    };
}

#[macro_export]
macro_rules! map_or_fn {
    (fn $func:expr; $name:expr) => {
        $crate::token::MapOrFn::Fn {
            func: Box::new($func),
            name: $name.into(),
        }
    };
    (fn $func:expr) => {
        $crate::map_or_fn!(fn $func; stringify!($func))
    };
    ($($key:expr => $value:expr),*$(,)?) => {
        $crate::token::MapOrFn::Map($crate::map!($($key => $value),*))
    };
}

#[macro_export]
macro_rules! set {
    ($($elem:expr),*$(,)?) => {
        {
            let _cap = $crate::_count!($($elem),*);
            let mut _set = std::collections::HashSet::with_capacity(_cap);
            $(
                _set.insert($elem);
            )*
            _set
        }
    };
}

#[macro_export]
macro_rules! set_or_fn {
    (fn $func:expr; $name:expr) => {
        $crate::token::SetOrFn::Fn {
            func: Box::new($func),
            name: $name.into(),
        }
    };
    (fn $func:expr) => {
        $crate::set_or_fn!(fn $func; stringify!($func))
    };
    ($($elem:expr),*$(,)?) => {
        $crate::token::SetOrFn::Set($crate::set!($($elem),*))
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __count {
    (@single $($x:tt)*) => (());
    ($($rest:expr),*) => (<[()]>::len(&[$($crate::__count!(@single $rest)),*]));
}

#[cfg(test)]
mod tests {
    #[test]
    fn macros() {
        let func_1 = set_or_fn!(fn char::is_ascii);
        let func_2: super::SetOrFn<char> = set_or_fn!(fn |_| true);
        let func_3: super::SetOrFn<char> = set_or_fn!(fn |_| true; "any");
        let map_1 = map_or_fn!('a' => 1, 'b' => 2);

        assert_eq!(
            func_1,
            super::SetOrFn::Fn {
                func: Box::new(char::is_ascii),
                name: "char::is_ascii".into()
            }
        );

        assert_eq!(
            func_2,
            super::SetOrFn::Fn {
                func: Box::new(|_| true),
                name: "|_| true".into()
            }
        );

        assert_eq!(
            func_3,
            super::SetOrFn::Fn {
                func: Box::new(|_| true),
                name: "any".into()
            }
        );

        assert_eq!(
            map_1,
            super::MapOrFn::Map([('a', 1), ('b', 2)].iter().cloned().collect()),
        );
    }
}
