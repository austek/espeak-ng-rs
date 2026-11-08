// tests/french_liaison_glide.rs
//
// ph_french keeps the /z/ of z2 and the /t/ of t2 before a following `j`
// ("deux yeux", "trois ème"), and the /z/ of z2/z3 before `w/`; the port kept
// a liaison consonant only before a vowel.  Expected output is from C
// espeak-ng master.

use espeak_ng::Translator;

#[test]
fn z_and_t_liaison_surface_before_a_glide() {
    let Ok(t) = Translator::new("fr", None) else {
        eprintln!("[SKIP] no fr data");
        return;
    };
    for (text, expected) in [
        ("deux yeux", "dˈøz jˈø"),
        ("trois ème", "tʁwˈaz jˈɛm"),
        ("3 ème", "tʁwˈaz jˈɛm"),
        ("2 ème", "dˈøz jˈɛm"),
        ("trois hier", "tʁwˈa jˈɛʁ"),
    ] {
        assert_eq!(t.text_to_ipa(text).unwrap(), expected, "fr {text}");
    }
}
