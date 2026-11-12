//! Drives the C API the way the native shells do, including freeing everything it returns.

use liyamu::*;
use std::ffi::CStr;

unsafe fn s(p: *mut std::ffi::c_char) -> String {
    CStr::from_ptr(p).to_str().unwrap().to_string()
}

#[test]
fn type_a_word_in_fm() {
    unsafe {
        let opts = LiyamuOptions { layout: 0, output: 1, order: 0, pali: false, suggest: true, repaya: false, conjuncts: false, pali_words: true };
        let sess = liyamu_session_new(&opts);
        let mut shown = String::new();
        for c in "fld".chars() {
            let r = liyamu_session_key(sess, LIYAMU_KEY_CHAR, c as u32, false, false);
            assert!((*r).handled);
            shown = s((*r).preedit);
            liyamu_response_free(r);
        }
        assert_eq!(shown, "fld");
        let r = liyamu_session_key(sess, LIYAMU_KEY_SPACE, 0, false, false);
        assert_eq!(s((*r).commit), "fld ");
        assert_eq!(s((*r).preedit), "");
        liyamu_response_free(r);
        // nothing composed: Backspace is the app's
        let r = liyamu_session_key(sess, LIYAMU_KEY_BACKSPACE, 0, false, false);
        assert!(!(*r).handled);
        liyamu_response_free(r);
        // Cmd/Ctrl shortcuts commit and pass through
        liyamu_response_free(liyamu_session_key(sess, LIYAMU_KEY_CHAR, 'l' as u32, false, false));
        let r = liyamu_session_key(sess, LIYAMU_KEY_CHAR, 'c' as u32, false, true);
        assert!(!(*r).handled);
        assert_eq!(s((*r).commit), "l");
        liyamu_response_free(r);
        // switching to Easy Singlish, Unicode
        let opts = LiyamuOptions { layout: 1, output: 0, order: 0, pali: false, suggest: true, repaya: false, conjuncts: false, pali_words: true };
        liyamu_response_free(liyamu_session_set_options(sess, &opts));
        for c in "amm".chars() {
            liyamu_response_free(liyamu_session_key(sess, LIYAMU_KEY_CHAR, c as u32, false, false));
        }
        let r = liyamu_session_key(sess, LIYAMU_KEY_CHAR, 'a' as u32, false, false);
        assert!((*r).candidate_count >= 2, "suggestions");
        let second = s(*(*r).candidates.add(1));
        liyamu_response_free(r);
        let r = liyamu_session_commit(sess);
        assert_eq!(s((*r).commit), "අම්මා");
        liyamu_response_free(r);
        // picking a suggestion commits it
        for c in "amma".chars() {
            liyamu_response_free(liyamu_session_key(sess, LIYAMU_KEY_CHAR, c as u32, false, false));
        }
        let r = liyamu_session_pick(sess, 1);
        assert_eq!(s((*r).commit), second);
        liyamu_response_free(r);
        // Helakuru keys
        let opts = LiyamuOptions { layout: 2, output: 0, order: 0, pali: false, suggest: true, repaya: false, conjuncts: false, pali_words: true };
        liyamu_response_free(liyamu_session_set_options(sess, &opts));
        for c in "dha".chars() {
            liyamu_response_free(liyamu_session_key(sess, LIYAMU_KEY_CHAR, c as u32, false, false));
        }
        let r = liyamu_session_commit(sess);
        assert_eq!(s((*r).commit), "ද");
        liyamu_response_free(r);
        liyamu_session_free(sess);
    }
}
