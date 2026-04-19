use rtoolbox::safe_string::SafeString;
use crate::config::PasswordFeedback;

pub struct FeedbackState {
    password: SafeString,
    needs_terminal_configuration: bool,
    displayed_count: usize,
    feedback: PasswordFeedback,
}

impl FeedbackState {
    pub fn new(feedback: PasswordFeedback, needs_terminal_configuration: bool) -> Self {
        FeedbackState {
            password: SafeString::new(),
            needs_terminal_configuration,
            displayed_count: 0,
            feedback,
        }
    }

    pub fn push_char(&mut self, c: char) -> Vec<u8> {
        self.password.push(c);

        if !self.needs_terminal_configuration {
            return Vec::new();
        }

        match self.feedback {
            PasswordFeedback::Hide => Vec::new(),
            PasswordFeedback::Mask(mask) => {
                self.displayed_count += 1;
                char_to_bytes(mask)
            }
            PasswordFeedback::PartialMask(mask, n) => {
                self.displayed_count += 1;
                if self.displayed_count <= n {
                    char_to_bytes(c)
                } else {
                    char_to_bytes(mask)
                }
            }
        }
    }

    pub fn pop_char(&mut self) -> Vec<u8> {
        let last_char = self.password.chars().last();
        if let Some(c) = last_char {
            let new_len = self.password.len() - c.len_utf8();
            self.password.truncate(new_len);

            if !self.needs_terminal_configuration {
                return Vec::new();
            }

            if self.displayed_count > 0 {
                self.displayed_count -= 1;
                vec![0x08, b' ', 0x08]
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        }
    }

    pub fn clear(&mut self) -> Vec<u8> {
        self.password = SafeString::new();

        if !self.needs_terminal_configuration {
            return Vec::new();
        }

        let count = self.displayed_count;
        self.displayed_count = 0;
        [0x08u8, b' ', 0x08].repeat(count)
    }

    pub fn abort(&mut self) -> Vec<u8> {
        self.password = SafeString::new();

        if !self.needs_terminal_configuration {
            return Vec::new();
        }

        self.displayed_count = 0;
        [b'\n'].to_vec()
    }

    pub fn finish(&mut self) -> Vec<u8> {
        if !self.needs_terminal_configuration {
            return Vec::new();
        }

        [b'\n'].to_vec()
    }

    pub fn is_empty(&self) -> bool {
        self.password.is_empty()
    }

    pub fn into_password(self) -> String {
        self.password.into_inner()
    }
}

fn char_to_bytes(c: char) -> Vec<u8> {
    let mut buf = [0u8; 4];
    c.encode_utf8(&mut buf).as_bytes().to_vec()
}

#[cfg(test)]
mod tests {
    mod with_terminal_configuration {
        use crate::config::PasswordFeedback;
        use crate::feedback::FeedbackState;

        #[test]
        fn feedback_state_mask_star() {
            let mut state = FeedbackState::new(PasswordFeedback::Mask('*'), true);
            assert_eq!(state.push_char('a'), b"*");
            assert_eq!(state.push_char('b'), b"*");
            assert_eq!(state.push_char('c'), b"*");
            assert_eq!(state.pop_char(), vec![0x08, b' ', 0x08]);
            assert_eq!(state.into_password(), "ab");
        }

        #[test]
        fn feedback_state_mask_hash() {
            let mut state = FeedbackState::new(PasswordFeedback::Mask('#'), true);
            assert_eq!(state.push_char('x'), b"#");
            assert_eq!(state.push_char('y'), b"#");
            assert_eq!(state.into_password(), "xy");
        }

        #[test]
        fn feedback_state_hide() {
            let mut state = FeedbackState::new(PasswordFeedback::Hide, true);
            assert!(state.push_char('a').is_empty());
            assert!(state.push_char('b').is_empty());
            assert!(state.pop_char().is_empty());
            assert_eq!(state.into_password(), "a");
        }

        #[test]
        fn feedback_state_partial_mask() {
            let mut state = FeedbackState::new(PasswordFeedback::PartialMask('*', 3), true);
            assert_eq!(state.push_char('a'), b"a");
            assert_eq!(state.push_char('b'), b"b");
            assert_eq!(state.push_char('c'), b"c");
            assert_eq!(state.push_char('d'), b"*");
            assert_eq!(state.push_char('e'), b"*");
            assert_eq!(state.into_password(), "abcde");
        }

        #[test]
        fn feedback_state_backspace_empty() {
            let mut state = FeedbackState::new(PasswordFeedback::Mask('*'), true);
            assert!(state.pop_char().is_empty());
        }

        #[test]
        fn feedback_state_clear() {
            let mut state = FeedbackState::new(PasswordFeedback::Mask('*'), true);
            state.push_char('a');
            state.push_char('b');
            state.push_char('c');
            assert_eq!(state.clear(), [0x08u8, b' ', 0x08].repeat(3));
            assert!(state.is_empty());
        }

        #[test]
        fn feedback_state_abort() {
            let mut state = FeedbackState::new(PasswordFeedback::Mask('*'), true);
            state.push_char('a');
            state.push_char('b');
            state.push_char('c');
            assert_eq!(state.abort(), [b'\n']);
            assert!(state.is_empty());
        }

        #[test]
        fn feedback_state_finish() {
            let mut state = FeedbackState::new(PasswordFeedback::Mask('*'), true);
            state.push_char('a');
            state.push_char('b');
            state.push_char('c');
            assert_eq!(state.finish(), [b'\n']);
            assert_eq!(state.into_password(), "abc");
        }

        #[test]
        fn feedback_state_partial_mask_zero() {
            let mut state = FeedbackState::new(PasswordFeedback::PartialMask('*', 0), true);
            assert_eq!(state.push_char('a'), b"*");
            assert_eq!(state.push_char('b'), b"*");
            assert_eq!(state.into_password(), "ab");
        }
    }

    mod without_terminal_configuration {
        use crate::config::PasswordFeedback;
        use crate::feedback::FeedbackState;

        #[test]
        fn feedback_state_mask_star() {
            let mut state = FeedbackState::new(PasswordFeedback::Mask('*'), false);
            assert_eq!(state.push_char('a'), vec![]);
            assert_eq!(state.push_char('b'), vec![]);
            assert_eq!(state.push_char('c'), vec![]);
            assert_eq!(state.pop_char(), vec![]);
            assert_eq!(state.into_password(), "ab");
        }

        #[test]
        fn feedback_state_mask_hash() {
            let mut state = FeedbackState::new(PasswordFeedback::Mask('#'), false);
            assert_eq!(state.push_char('x'), vec![]);
            assert_eq!(state.push_char('y'), vec![]);
            assert_eq!(state.into_password(), "xy");
        }

        #[test]
        fn feedback_state_hide() {
            let mut state = FeedbackState::new(PasswordFeedback::Hide, false);
            assert!(state.push_char('a').is_empty());
            assert!(state.push_char('b').is_empty());
            assert!(state.pop_char().is_empty());
            assert_eq!(state.into_password(), "a");
        }

        #[test]
        fn feedback_state_partial_mask() {
            let mut state = FeedbackState::new(PasswordFeedback::PartialMask('*', 3), false);
            assert_eq!(state.push_char('a'), vec![]);
            assert_eq!(state.push_char('b'), vec![]);
            assert_eq!(state.push_char('c'), vec![]);
            assert_eq!(state.push_char('d'), vec![]);
            assert_eq!(state.push_char('e'), vec![]);
            assert_eq!(state.into_password(), "abcde");
        }

        #[test]
        fn feedback_state_backspace_empty() {
            let mut state = FeedbackState::new(PasswordFeedback::Mask('*'), false);
            assert!(state.pop_char().is_empty());
        }

        #[test]
        fn feedback_state_clear() {
            let mut state = FeedbackState::new(PasswordFeedback::Mask('*'), false);
            state.push_char('a');
            state.push_char('b');
            state.push_char('c');
            assert_eq!(state.clear(), vec![]);
            assert!(state.is_empty());
        }

        #[test]
        fn feedback_state_abort() {
            let mut state = FeedbackState::new(PasswordFeedback::Mask('*'), false);
            state.push_char('a');
            state.push_char('b');
            state.push_char('c');
            assert_eq!(state.abort(), vec![]);
            assert!(state.is_empty());
        }

        #[test]
        fn feedback_state_finish() {
            let mut state = FeedbackState::new(PasswordFeedback::Mask('*'), false);
            state.push_char('a');
            state.push_char('b');
            state.push_char('c');
            assert_eq!(state.finish(), vec![]);
            assert_eq!(state.into_password(), "abc");
        }

        #[test]
        fn feedback_state_partial_mask_zero() {
            let mut state = FeedbackState::new(PasswordFeedback::PartialMask('*', 0), false);
            assert_eq!(state.push_char('a'), vec![]);
            assert_eq!(state.push_char('b'), vec![]);
            assert_eq!(state.into_password(), "ab");
        }
    }
}