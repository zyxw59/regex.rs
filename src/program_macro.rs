#[macro_export]
macro_rules! program {
    ($($(:$label:ident)? $instr:ident $(($($args:tt)*))?,)*) => {{
        let mut count = 0;
        $(
            $(let $label = count;)?
            count += 1;
        )*
        let mut prog = Vec::with_capacity(count);
        let mut max_slot = 0;
        $(
            let instr = $crate::instruction!($instr $(($($args)*))?, max_slot);
            prog.push(instr);
        )*
        $crate::program::Program::<_, $crate::state::SaveList>::new(prog, max_slot + 1)
    }};
}

#[macro_export]
macro_rules! instruction {
    (Map($($tok:expr => $label:expr),*), $max_slot:ident) => {
        $crate::program::Instr::Map([$(($tok, $label)),*].into_iter().collect())
    };
    (Set($($tok:expr),*), $max_slot:ident) => {
        $crate::program::Instr::Set([$($tok),*].into_iter().collect())
    };
    (UpdateState($slot:expr), $max_slot:ident) => {{
        $max_slot = $max_slot.max($slot);
        $crate::program::Instr::UpdateState($slot)
    }};
    // Any, WordBoundary, Reject, Match
    ($instr:tt, $max_slot:ident) => {
        $crate::program::Instr::$instr
    };
    // Token, Split, JSplit, Jump
    ($instr:tt ($arg:expr), $max_slot:ident) => {
        $crate::program::Instr::$instr($arg)
    };
}

#[cfg(test)]
#[test]
fn program() {
    use crate::program::Instr::*;

    let program_macro = program![
        // /(ab?)(b?c)\b/
        // match .*? before start of match
        :l0 JSplit(l1),
        Any,
        Jump(l0),
        // start of match
        :l1 UpdateState(0),
        // start of first subgroup
        UpdateState(2),
        Token('a'),
        // b?
        Split(l2),
        Token('b'),
        // end of first subgroup
        :l2 UpdateState(3),
        // start of second subgroup
        UpdateState(4),
        // b?
        Split(l3),
        Token('b'),
        :l3 Token('c'),
        // end of second subgroup
        UpdateState(5),
        WordBoundary,
        // end of match
        UpdateState(1),
        Match,
    ];

    let prog = vec![
        // 0: *? quantifier
        JSplit(3),
        // 1: match a token
        Any,
        // 2: repeat
        Jump(0),
        // 3: save start of match
        UpdateState(0),
        // 4: save start of first subgroup
        UpdateState(2),
        // 5: a
        Token('a'),
        // 6: optional b
        Split(8),
        // 7: b
        Token('b'),
        // 8: save end of first subgroup
        UpdateState(3),
        // 9: save start of second subgroup
        UpdateState(4),
        // 10: optional b
        Split(12),
        // 11: b
        Token('b'),
        // 12: c
        Token('c'),
        // 13: save end of second subgroup
        UpdateState(5),
        // 14: word boundary
        WordBoundary,
        // 15: save end of match
        UpdateState(1),
        // 16: end of match
        Match,
    ];
    let num_slots = 6;
    let program_expected = crate::program::Program::new(prog, num_slots);
    assert_eq!(program_macro, program_expected);
}
