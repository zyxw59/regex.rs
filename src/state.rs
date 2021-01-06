use crate::token::Token;

/// Trait for any state that the engine needs to store.
pub trait State<T: Token>: std::hash::Hash + Eq + Clone {
    /// Input to the state's update instruction.
    type Update;

    /// Input to the state's initializer.
    type Init;

    fn init(init: &Self::Init) -> Self;

    /// Update the state, returning `true` if the update was successful, and
    /// `false` if not (rejecting the match).
    fn update<'a>(&mut self, update: &'a Self::Update, program_state: ProgramState<'a, T>) -> bool;
}

/// State about the current execution of the program, which is not specific to
/// a specific `State` implementation.
#[derive(Debug)]
pub struct ProgramState<'a, T: 'a> {
    pub instruction_ptr: crate::program::InstrPtr,
    pub token_index: usize,
    pub token: Option<&'a T>,
}

// need manual impls here, since #[derive(Copy)] inserts a `where T: Copy`
// bound which we don't want.
impl<'a, T: 'a> Copy for ProgramState<'a, T> {}
impl<'a, T: 'a> Clone for ProgramState<'a, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, T: 'a> ProgramState<'a, T> {
    pub fn increment_ip(mut self) -> Self {
        self.instruction_ptr += 1;
        self
    }

    pub fn with_ip(mut self, new_ip: crate::program::InstrPtr) -> Self {
        self.instruction_ptr = new_ip;
        self
    }

    pub fn optional_with_ip(self, new_ip: Option<crate::program::InstrPtr>) -> Self {
        if let Some(new_ip) = new_ip {
            self.with_ip(new_ip)
        } else {
            self
        }
    }
}

/// A list of saved indices into the search string.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct SaveList(pub Vec<Option<usize>>);

impl<T: Token> State<T> for SaveList {
    /// The slot to save into.
    type Update = usize;

    /// The number of save slots
    type Init = usize;

    fn init(init: &usize) -> Self {
        SaveList(vec![None; *init])
    }

    fn update<'a>(&mut self, update: &'a usize, program_state: ProgramState<'a, T>) -> bool {
        if let Some(slot) = self.0.get_mut(*update) {
            *slot = Some(program_state.token_index);
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    struct Concat(String);

    impl State<char> for Concat {
        type Update = ();
        type Init = ();

        fn init(_: &()) -> Self {
            Concat(String::new())
        }

        fn update<'a>(&mut self, _: &'a (), program_state: ProgramState<'a, char>) -> bool {
            self.0.push(*program_state.token.unwrap_or(&'\0'));
            true
        }
    }

    #[test]
    fn state_updates() {
        use crate::program::Instr::*;
        let program = crate::program::Program::<char, Concat>::new(
            vec![
                UpdateState(()),
                Any,
                Split(0),
                UpdateState(()),
                UpdateState(()),
                Match,
            ],
            (),
        );

        let result = program.exec("abc");
        assert_eq!(
            result,
            &[
                Concat("\0aa".into()),
                Concat("\0abb".into()),
                Concat("\0abcc".into())
            ]
        );
    }
}
