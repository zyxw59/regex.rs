#[cfg(test)]
mod tests {
    use super::ast;

    fn index_match<'a, H>(
        haystack: &'a H,
        saves: &[crate::program::SaveList],
        match_: usize,
        subgroup: usize,
    ) -> &'a H::Output
    where
        H: std::ops::Index<std::ops::Range<usize>> + ?Sized,
    {
        let start = saves[match_][subgroup * 2].unwrap();
        let end = saves[match_][subgroup * 2 + 1].unwrap();
        &haystack[start..end]
    }

    #[test]
    fn program() {
        let program = super::program![
            // /(ab?)(b?c)\b/
            // match .*? before start of match
            :l0 JSplit(l1),
            Any,
            Jump(l0),
            // start of match
            :l1 Save(0),
            // start of first subgroup
            Save(2),
            Token('a'),
            // b?
            Split(l2),
            Token('b'),
            // end of first subgroup
            :l2 Save(3),
            // start of second subgroup
            Save(4),
            // b?
            Split(l3),
            Token('b'),
            :l3 Token('c'),
            // end of second subgroup
            Save(5),
            WordBoundary,
            // end of match
            Save(1),
            Match,
        ];
        let haystack = "ducabc";
        let saves = program.exec(haystack);
        assert_eq!(index_match(haystack, &saves, 0, 0), "abc");
        assert_eq!(index_match(haystack, &saves, 0, 1), "ab");
        assert_eq!(index_match(haystack, &saves, 0, 2), "c");
        assert_eq!(index_match(haystack, &saves, 1, 0), "abc");
        assert_eq!(index_match(haystack, &saves, 1, 1), "a");
        assert_eq!(index_match(haystack, &saves, 1, 2), "bc");
        assert_eq!(
            saves,
            vec![
                vec![Some(3), Some(6), Some(3), Some(5), Some(5), Some(6)],
                vec![Some(3), Some(6), Some(3), Some(4), Some(4), Some(6)],
            ]
        );
    }

    #[test]
    fn program_unicode() {
        let program = super::program![
            // (.)(.)(.)
            // match .*? before start of match
            :l0 JSplit(l1),
            Any,
            Jump(l0),
            // start of match
            :l1 Save(0),
            // first subgroup
            Save(2),
            Any,
            Save(3),
            // second subgroup
            Save(4),
            Any,
            Save(5),
            // third subgroup
            Save(6),
            Any,
            Save(7),
            // end of match
            Save(1),
            Match,
        ];
        // the search string contains characters with 1, 2, 3, and 4 byte representations,
        // respectively (U+0024, U+00A2, U+20AC, U+10348)
        let haystack = "$¢€𐍈";
        let saves = program.exec(haystack);
        assert_eq!(index_match(haystack, &saves, 0, 0), "$¢€");
        assert_eq!(index_match(haystack, &saves, 0, 1), "$");
        assert_eq!(index_match(haystack, &saves, 0, 2), "¢");
        assert_eq!(index_match(haystack, &saves, 0, 3), "€");
        assert_eq!(index_match(haystack, &saves, 1, 0), "¢€𐍈");
        assert_eq!(index_match(haystack, &saves, 1, 1), "¢");
        assert_eq!(index_match(haystack, &saves, 1, 2), "€");
        assert_eq!(index_match(haystack, &saves, 1, 3), "𐍈");

        // with vec instead of &str
        let haystack = "$¢€𐍈".chars().collect::<Vec<char>>();
        let saves = program.exec(&*haystack);
        assert_eq!(index_match(&haystack, &saves, 0, 0), &['$', '¢', '€']);
        assert_eq!(index_match(&haystack, &saves, 0, 1), &['$']);
        assert_eq!(index_match(&haystack, &saves, 0, 2), &['¢']);
        assert_eq!(index_match(&haystack, &saves, 0, 3), &['€']);
        assert_eq!(index_match(&haystack, &saves, 1, 0), &['¢', '€', '𐍈']);
        assert_eq!(index_match(&haystack, &saves, 1, 1), &['¢']);
        assert_eq!(index_match(&haystack, &saves, 1, 2), &['€']);
        assert_eq!(index_match(&haystack, &saves, 1, 3), &['𐍈']);
    }

    #[test]
    fn ast() {
        use crate::ast::Regex::*;
        // /(ab?)(b?c)a\b/
        let tree = Concat(vec![
            Capture(Box::new(Concat(vec![
                Literal(vec!['a']),
                Repeat(Box::new(Literal(vec!['b'])), ast::Repeater::ZeroOrOne(true)),
            ]))),
            Capture(Box::new(Concat(vec![
                Repeat(Box::new(Literal(vec!['b'])), ast::Repeater::ZeroOrOne(true)),
                Literal(vec!['c']),
            ]))),
            WordBoundary,
        ]);
        let prog = tree.compile();
        let saves = prog.exec("ducabc ");
        assert_eq!(
            saves,
            vec![
                vec![Some(3), Some(6), Some(3), Some(5), Some(5), Some(6)],
                vec![Some(3), Some(6), Some(3), Some(4), Some(4), Some(6)],
            ]
        );
    }
}

pub mod ast;
pub mod program;
pub mod program_macro;
pub mod searcher;
pub mod token;
