//! Property tests: any word poman does not know is refused, with nothing on
//! standard output.

use proptest::prelude::{prop_assert, prop_assert_eq};
use proptest::test_runner::{TestError, TestRunner};

#[test]
fn an_unknown_word_is_refused() -> Result<(), TestError<String>> {
    TestRunner::default().run(&"[a-z][a-z0-9-]{0,16}", |word| {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = poman::run(["poman", word.as_str()], &mut out, &mut err);
        prop_assert_eq!(code, 2);
        prop_assert!(out.is_empty());
        prop_assert!(!err.is_empty());
        Ok(())
    })
}
