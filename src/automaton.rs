use std::borrow::Cow;

use fst::Automaton;

pub enum PrefixAutomatonState {
    Sink,
    State(usize),
}

/// An automaton that matches every prefix of a string. For
/// example, an automaton that matches *"hello"* will match:
/// *""*, *"h"*, *"he"*, *"hel"*, *"hell"*, *"hello"*.
///
/// Only prefixes consisting of complete code points are
/// recognized. For example, for the string *"ë"*, *[]*
/// and *[0xc3, 0xab]* are accepted, whereas *[0xc3]* is
/// not.
pub struct PrefixAutomaton<'a>(Cow<'a, str>);

impl<'a> Automaton for PrefixAutomaton<'a> {
    type State = PrefixAutomatonState;

    fn start(&self) -> Self::State {
        PrefixAutomatonState::State(0)
    }

    fn can_match(&self, state: &Self::State) -> bool {
        match *state {
            PrefixAutomatonState::Sink => false,
            PrefixAutomatonState::State(_) => true,
        }
    }

    fn is_match(&self, state: &Self::State) -> bool {
        match *state {
            PrefixAutomatonState::Sink => false,
            PrefixAutomatonState::State(idx) => self.0.is_char_boundary(idx),
        }
    }

    fn accept(&self, state: &Self::State, byte: u8) -> Self::State {
        match *state {
            PrefixAutomatonState::Sink => PrefixAutomatonState::Sink,
            PrefixAutomatonState::State(idx) => {
                if idx == self.0.len() {
                    // Recognizing characters beyond the end of the string
                    // leads to the sink state.
                    PrefixAutomatonState::Sink
                } else if self.0.as_bytes()[idx] == byte {
                    // Move to the next state if the byte is recognized.
                    PrefixAutomatonState::State(idx + 1)
                } else {
                    // Otherwise, the byte is not recognized and we move
                    // to the sink state.
                    PrefixAutomatonState::Sink
                }
            }
        }
    }
}

impl From<String> for PrefixAutomaton<'static> {
    fn from(s: String) -> Self {
        PrefixAutomaton(Cow::Owned(s))
    }
}

impl<'a> From<&'a str> for PrefixAutomaton<'a> {
    fn from(s: &'a str) -> Self {
        PrefixAutomaton(Cow::Borrowed(s))
    }
}

#[cfg(test)]
mod tests {
    use fst::Automaton;
    use quickcheck::quickcheck;

    use super::PrefixAutomaton;

    fn automaton_match<A>(automaton: A, data: &[u8]) -> bool
    where
        A: Automaton,
    {
        let mut state = automaton.start();

        for byte in data {
            state = automaton.accept(&state, *byte);
        }

        automaton.is_match(&state)
    }

    /// Check that the automaton only matches on codepoint boundaries.
    #[test]
    fn do_not_match_incomplete_prefix_test() {
        // UTF-8 encoding: [0xc3, 0xa4, 0xc3, 0xab]
        let s = "äë";
        let s_bytes = s.as_bytes();

        let automaton = PrefixAutomaton::from(s);

        assert!(automaton_match(&automaton, &s_bytes[..0]));
        assert!(!automaton_match(&automaton, &s_bytes[..1]));
        assert!(automaton_match(&automaton, &s_bytes[..2]));
        assert!(!automaton_match(&automaton, &s_bytes[..3]));
        assert!(automaton_match(&automaton, &s_bytes[..4]));
    }

    quickcheck! {
        /// Check that all prefixes of a string are matched by its
        /// prefix automaton.
        fn prefix_matches_prop(s: String) -> bool {
            let automaton = PrefixAutomaton::from(s.as_str());
            let chars: Vec<_> = s.chars().collect();

            for idx in 0..(chars.len() + 1) {
                let prefix = &chars[..idx];
                if !automaton_match(&automaton, prefix.iter().cloned().collect::<String>().as_bytes()) {
                    return false;
                }
            }

            true
        }
    }

    quickcheck! {
        /// Check prefixes on arbitrary strings.
        fn random_string_prefix_matches_prop(s1: String, s2: String) -> bool {
            let automaton = PrefixAutomaton::from(s1.as_str());

            let chars1: Vec<_> = s1.chars().collect();
            let chars2: Vec<_> = s2.chars().collect();

            for idx in 0..(chars2.len() + 1) {
                let prefix = &chars2[..idx];

                if !automaton_match(&automaton, prefix.iter().cloned().collect::<String>().as_bytes()) && chars1.starts_with(prefix) {
                    return false;
                }
            }

            true
        }
    }
}
