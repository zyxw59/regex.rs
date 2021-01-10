pub trait Searcher {
    type Item;

    fn next(&mut self) -> (usize, Option<Self::Item>);
}

pub struct StrSearcher<'a> {
    next_index: usize,
    iter: std::str::Chars<'a>,
}

impl<'a> Searcher for StrSearcher<'a> {
    type Item = char;

    fn next(&mut self) -> (usize, Option<char>) {
        let c = self.iter.next();
        if let Some(c) = c {
            self.next_index += c.len_utf8();
        }
        (self.next_index, c)
    }
}

pub struct IterSearcher<I> {
    next_index: usize,
    iter: I,
}

impl<I: Iterator> IterSearcher<I> {
    pub fn new(iter: I) -> Self {
        IterSearcher {
            next_index: 0,
            iter,
        }
    }
}

impl<I: Iterator> Searcher for IterSearcher<I> {
    type Item = I::Item;

    fn next(&mut self) -> (usize, Option<I::Item>) {
        let t = self.iter.next();
        if t.is_some() {
            self.next_index += 1;
        }
        (self.next_index, t)
    }
}

pub trait IntoSearcher<T> {
    type Searcher: Searcher<Item = T>;

    fn into_searcher(self) -> Self::Searcher;
}

impl<'a> IntoSearcher<char> for &'a str {
    type Searcher = StrSearcher<'a>;

    fn into_searcher(self) -> Self::Searcher {
        StrSearcher {
            next_index: 0,
            iter: self.chars(),
        }
    }
}

impl<'a, T> IntoSearcher<&'a T> for &'a [T] {
    type Searcher = IterSearcher<std::slice::Iter<'a, T>>;

    fn into_searcher(self) -> Self::Searcher {
        IterSearcher::new(self.iter())
    }
}

impl<T> IntoSearcher<T> for Vec<T> {
    type Searcher = IterSearcher<std::vec::IntoIter<T>>;

    fn into_searcher(self) -> Self::Searcher {
        IterSearcher::new(self.into_iter())
    }
}

impl<S: Searcher<Item = T>, T> IntoSearcher<T> for S {
    type Searcher = S;

    fn into_searcher(self) -> Self::Searcher {
        self
    }
}
