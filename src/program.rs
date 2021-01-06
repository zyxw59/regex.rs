use std::borrow::Borrow;
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::mem;
use std::ops::Index;

use crate::searcher::{IntoSearcher, Searcher};
use crate::state::{ProgramState, State};
use crate::token::Token;

/// Type for indexing into a program
pub type InstrPtr = usize;

/// A single instruction
pub enum Instr<T: Token, S: State<T>> {
    /// Matches a single token.
    Token(T),
    /// Matches any token.
    Any,
    /// Maps input tokens to new `InstrPtr`s. If the input token is not found, falls through to the
    /// next instruction.
    Map(HashMap<T, InstrPtr>),
    /// Matches a single token from a set of tokens.
    Set(HashSet<T>),
    /// Matches a word boundary.
    WordBoundary,
    /// Splits into two states, preferring not to jump. Used to implement alternations and
    /// quantifiers
    Split(InstrPtr),
    /// Splits into two states, preferring to jump. Used to implement alternations and quantifiers.
    JSplit(InstrPtr),
    /// Jumps to a new point in the program.
    Jump(InstrPtr),
    /// Updates the thread state with the specified update arguments.
    ///
    /// For the `SaveList` state, this saves the current token index to the
    /// indicated save slot. This is used for subgroup matching. In general,
    /// save the start of the <i>n</i>th capturing group to slot _2n_, and the
    /// end to slot _2n + 1_.
    UpdateState(S::Update),
    /// Reject a potential match. Can be used after a Map when fallthrough should fail.
    Reject,
    /// The end of a match.
    Match,
}

#[cfg(test)]
impl<T, S> PartialEq for Instr<T, S>
where
    T: Token,
    S: State<T>,
    S::Update: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        use self::Instr::*;
        match (self, other) {
            (Any, Any) | (WordBoundary, WordBoundary) | (Reject, Reject) | (Match, Match) => true,
            (Map(s), Map(o)) => s == o,
            (Set(s), Set(o)) => s == o,
            (Token(s), Token(o)) => s == o,
            (Split(s), Split(o)) | (JSplit(s), JSplit(o)) | (Jump(s), Jump(o)) => s == o,
            (UpdateState(s), UpdateState(o)) => s == o,
            _ => false,
        }
    }
}

impl<T, S> Debug for Instr<T, S>
where
    T: Token,
    S: State<T>,
    S::Update: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use self::Instr::*;
        match self {
            Token(t) => write!(f, "Token({:?})", t),
            Any => write!(f, "Any"),
            Map(map) => write!(f, "Map({:?})", map),
            Set(set) => write!(f, "Set({:?})", set),
            WordBoundary => write!(f, "WordBoundary"),
            Split(ip) => write!(f, "Split({:?})", ip),
            JSplit(ip) => write!(f, "JSplit({:?})", ip),
            Jump(ip) => write!(f, "Jump({:?})", ip),
            UpdateState(update) => write!(f, "UpdateState({:?})", update),
            Reject => write!(f, "Reject"),
            Match => write!(f, "Match"),
        }
    }
}

/// A thread, consisting of an `InstrPtr` to the current instruction, and any
/// state used by the engine.
#[derive(Debug, Hash, Eq, PartialEq)]
struct Thread<S> {
    /// Pointer to current instruction
    pc: InstrPtr,
    /// Program state
    state: S,
}

impl<S> Thread<S> {
    /// Create a new `Thread` with the specified instruction pointer and the given state.
    fn new(pc: InstrPtr, state: S) -> Thread<S> {
        Thread { pc, state }
    }
}

/// A list of threads
#[derive(Debug)]
struct ThreadList<S> {
    threads: Vec<Thread<S>>,
}

impl<S> ThreadList<S> {
    /// Create a new `ThreadList` with a specified capacity
    fn new(cap: usize) -> ThreadList<S> {
        ThreadList {
            threads: Vec::with_capacity(cap),
        }
    }

    /// Add a new `Thread` with the specified instruction pointer, and the
    /// given state. If `pc` points to a `Jump`, `Split`, `JSplit`, or `UpdateState`
    /// instruction, calls `add_thread` recursively, so that the active
    /// `ThreadList` never contains pointers to those instructions.
    fn add_thread<T: Token>(
        &mut self,
        program_state: ProgramState<'_, T>,
        prog: &Program<T, S>,
        mut state: S,
    ) where
        S: State<T>,
    {
        // don't check if there's already a thread with this `pc` on the list, because we want to
        // keep alternate paths alive, in case they produce different submatch values.
        use self::Instr::*;
        match prog[program_state.instruction_ptr] {
            Split(split) => {
                // call `add_thread` recursively
                // branch with no jump is higher priority
                // clone the `state` so we can use it again in the second branch
                self.add_thread(program_state.increment_ip(), prog, state.clone());
                self.add_thread(program_state.with_ip(split), prog, state);
            }
            JSplit(split) => {
                // call `add_thread` recursively
                // branch with jump is higher priority
                // clone the `state` so we can use it again in the second branch
                self.add_thread(program_state.with_ip(split), prog, state.clone());
                self.add_thread(program_state.increment_ip(), prog, state);
            }
            Jump(jump) => {
                // call `add_thread` recursively
                // jump to specified pc
                self.add_thread(program_state.with_ip(jump), prog, state);
            }
            UpdateState(ref update) => {
                // update state
                state.update(update, program_state);
                // and recursively add next instruction
                self.add_thread(program_state.increment_ip(), prog, state);
            }
            Reject => {} // do nothing, this thread is dead
            Token(_) | Map(_) | Set(_) | Any | WordBoundary | Match => {
                // push a new thread with the given pc
                self.threads
                    .push(Thread::new(program_state.instruction_ptr, state));
            }
        }
    }
}

impl<'a, S> IntoIterator for &'a mut ThreadList<S> {
    type Item = Thread<S>;
    type IntoIter = ::std::vec::Drain<'a, Thread<S>>;

    fn into_iter(self) -> Self::IntoIter {
        self.threads.drain(..)
    }
}

/// A program for the VM
pub struct Program<T: Token, S: State<T>> {
    /// List of instructions. `InstrPtr`s are indexed into this vector
    prog: Vec<Instr<T, S>>,
    /// Initializer for state.
    state_init: S::Init,
}

impl<T: Token, S: State<T>> Program<T, S> {
    pub fn new(prog: Vec<Instr<T, S>>, state_init: S::Init) -> Program<T, S> {
        Program { prog, state_init }
    }

    /// Executes the program. Returns a vector of matches found. For each
    /// match, the final state for each match is returned.
    ///
    /// With the `SaveList` state, this means the location of each of the saved
    /// locations in the match.
    pub fn exec<U: Borrow<T>>(&self, input: impl IntoSearcher<U>) -> Vec<S> {
        self.exec_searcher(input.into_searcher())
    }

    /// Executes the program. Returns a vector of matches found. For each
    /// match, the final state for each match is returned.
    ///
    /// With the `SaveList` state, this means the location of each of the saved
    /// locations in the match.
    pub fn exec_iter<U: Borrow<T>>(&self, input: impl IntoIterator<Item = U>) -> Vec<S> {
        self.exec_searcher(crate::searcher::IterSearcher::new(input.into_iter()))
    }

    fn exec_searcher<U: Borrow<T>>(&self, mut searcher: impl Searcher<Item = U>) -> Vec<S> {
        // initialize thread lists. The number of threads should be limited by the length of the
        // program (since each instruction either ends a thread (in the case of a `Match` or a
        // failed `Token` instruction), continues an existing thread (in the case of a successful
        // `Token`, `Jump`, or `UpdateState` instruction), or spawns a new thread (in the case of a
        // `Split` or `JSplit` instruction))
        let mut curr = ThreadList::new(self.prog.len());
        let mut next = ThreadList::new(self.prog.len());

        let mut states = Vec::new();

        // start initial thread at start instruction
        curr.add_thread(
            ProgramState {
                instruction_ptr: 0,
                token_index: 0,
                token: None,
            },
            self,
            S::init(&self.state_init),
        );

        // set initial word flag
        let mut word = false;

        // to store the iteration number (declaring it here so it can be used in the final checks
        // after the loop)
        let mut i = 0;

        // iterate over tokens of input string
        while let (idx, Some(tok_i)) = searcher.next() {
            let tok_i = tok_i.borrow();
            // check if word boundary
            let new_word = tok_i.is_word();
            let word_boundary = new_word ^ word;
            word = new_word;

            // iterate over active threads, draining the list so we can reuse it without
            // reallocating
            for th in &mut curr {
                let next_program_state = ProgramState {
                    instruction_ptr: th.pc + 1,
                    token_index: idx,
                    token: Some(tok_i),
                };
                use self::Instr::*;
                match self[th.pc] {
                    Token(ref token) => {
                        // check if token matches
                        if tok_i == token {
                            // increment thread pc, passing along next input index, and state
                            next.add_thread(next_program_state, self, th.state);
                        }
                    }
                    Set(ref set) => {
                        // check if token in set
                        if set.contains(tok_i) {
                            // increment thread pc, passing along next input index, and state
                            next.add_thread(next_program_state, self, th.state);
                        }
                    }
                    Map(ref map) => {
                        // get the corresponding pc, or default to incrementing
                        next.add_thread(
                            next_program_state.optional_with_ip(map.get(tok_i).cloned()),
                            self,
                            th.state,
                        );
                    }
                    Any => {
                        // always matches
                        next.add_thread(next_program_state, self, th.state);
                    }
                    WordBoundary => {
                        // check if word boundary, don't consume character
                        if word_boundary {
                            next.add_thread(
                                ProgramState {
                                    instruction_ptr: th.pc + 1,
                                    token_index: i,
                                    token: Some(tok_i),
                                },
                                self,
                                th.state,
                            );
                        }
                    }
                    Match => {
                        // add the state to the final list
                        states.push(th.state);
                    }
                    // These instructions are handled in add_thread, so the current thread should
                    // never point to one of them
                    Split(_) | JSplit(_) | Jump(_) | UpdateState(_) | Reject => {
                        unreachable!();
                    }
                }
            }
            // `next` becomes list of active threads, and `curr` (empty after iteration) can hold
            // the next iteration
            mem::swap(&mut curr, &mut next);
            // increment`i`
            i = idx;
        }

        // now iterate over remaining threads, to check for pending word boundary instructions
        for th in &mut curr {
            use self::Instr::*;
            match self[th.pc] {
                WordBoundary => {
                    // check if last token was a word token
                    if word {
                        next.add_thread(
                            ProgramState {
                                instruction_ptr: th.pc + 1,
                                token_index: i,
                                token: None,
                            },
                            self,
                            th.state,
                        );
                    }
                }
                Match => {
                    states.push(th.state);
                }
                // anything else is a failed match
                _ => {}
            }
        }

        // now iterate over remaining threads, to check for pending match instructions
        for th in &mut next {
            // anything else is a failed match
            if let Instr::Match = self[th.pc] {
                states.push(th.state);
            }
        }

        // return the list of states
        states
    }
}

impl<T: Token, S: State<T>> Index<InstrPtr> for Program<T, S> {
    type Output = Instr<T, S>;

    fn index(&self, idx: InstrPtr) -> &Instr<T, S> {
        self.prog.index(idx)
    }
}

#[cfg(test)]
impl<T, S> PartialEq for Program<T, S>
where
    T: Token,
    S: State<T>,
    S::Update: PartialEq,
    S::Init: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.state_init == other.state_init && self.prog == other.prog
    }
}

impl<T, S> Debug for Program<T, S>
where
    T: Token,
    S: State<T>,
    S::Update: Debug,
    S::Init: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "Program {{")?;
        writeln!(f, "  state_init: {:?},", self.state_init)?;
        if self.prog.is_empty() {
            writeln!(f, "  prog: [],")?;
        } else {
            // not necessarily the fastest way to do this, but the time it
            // takes is negligible, and it works
            let max_len = (self.prog.len() - 1).to_string().len();
            writeln!(f, "  prog: [")?;
            for (idx, instr) in self.prog.iter().enumerate() {
                writeln!(f, "    {:>1$}: {2:?},", idx, max_len, instr)?;
            }
            writeln!(f, "  ],")?;
        }
        write!(f, "}}")
    }
}
