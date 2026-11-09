// `R$` is read through the rule `_) r (_\$ xe'aU` (the next word's `$` is
// visible to the post-context), and `°C` / `°F` stay "graus" plus the letter:
// upstream never expands the scale to Celsius or Fahrenheit.
//
// Values from a locally built upstream `espeak-ng` (`f0fb11a8`).
// Skips when the pt voice data is absent.

use espeak_ng::Translator;

#[test]
fn currency_sign_after_r_and_plain_degree_scales() {
    let Ok(t) = Translator::new("pt", None) else {
        eprintln!("[SKIP] no pt data");
        return;
    };
    for (text, want) in [
        ("R$", "ʁɨˈaʊ dˈolar"),
        ("R$ 5", "ʁɨˈaʊ dˈolar sˈiŋku"),
        ("5 R$", "sˈiŋku ʁɨˈaʊ dˈolar"),
        ("°C", "ɡɹˈaʊ sˈe"),
        ("20 °C", "vˈiŋtɨ ɡɹˈaʊ sˈe"),
        ("°F", "ɡɹˈaʊ ˈɛf"),
    ] {
        assert_eq!(&t.text_to_ipa(text).unwrap(), want, "{text:?}");
    }
}
