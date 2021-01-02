#[cfg(test)]
mod tests {
    use super::ast;

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
        let saves = program.exec("ducabc ".chars());
        assert_eq!(
            saves,
            vec![
                vec![Some(3), Some(6), Some(3), Some(5), Some(5), Some(6)],
                vec![Some(3), Some(6), Some(3), Some(4), Some(4), Some(6)],
            ]
        );
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
        let saves = prog.exec("ducabc ".chars());
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
pub mod token;
