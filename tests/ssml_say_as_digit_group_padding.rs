// tests/ssml_say_as_digit_group_padding.rs
//
// C `TranslateClause` cuts a long digit run into groups at `break_numbers`, writes the
// thousands separator after each group, and pads a group that is followed by a shorter
// one with zeros, so each pad is spoken as a digit.  Expected values are what C espeak-ng
// master (f0fb11a8) prints for `tts:digits`.

use std::process::Command;

fn run(lang: &str, flag: &str, text: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", flag, "-m", "-v", lang, text])
        .output()
        .expect("run");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

const CASES: [(&str, &str, &str, &str); 9] = [
    ("es", "12345", "ˈunoðˈospunto tɾˈeskwˈatɾoθˈinko", "'unoD'os_!punto t**'eskw'at**o_!T'inko"),
    (
        "es",
        "1234567",
        "ˈunopˌunto ðˈostɾˈeskwˈatɾopˌunto θˈinkosˈeɪssjˈete",
        "'unop,unto D'ost**'es_!kw'at**op,unto T'inkos'eIs_!sj'ete",
    ),
    (
        "nl",
        "1234567",
        "ˈeːnpɵn tʋˈeːdrˌivˌirpɵnt vˈɛɪfzˌɛszˌeːvən",
        "_!'e:np8n! tv#'e:dr,i_!v,irp8nt v'EIfz,Es_!z,e:v@n",
    ),
    (
        "pt",
        "1234567",
        "ˈumpˈoŋtʊ dˈoɪʃtɹˈeʃkwˈatɹupˈoŋtʊ sˈiŋkusˈeɪʃsˈɛtɨ",
        "'ump'oNtU d'oIs#tR'es#_!kw'atrup'oNtU s'iNkus'eIs#_!s'Ety",
    ),
    (
        "hi",
        "1234567",
        "ˈeːkdˈoː ʃˈuːnjətˈiːncˈaːɾ pˈa\u{303}ccʰˈʌhsˈaːt",
        "'e:kd'o:_! S'u:nj@-t'i:n_!c'a:r p'a~cc#'VH_!s'a:t",
    ),
    (
        "hi",
        "123456789",
        "ˈeːkdˈoː ʃˈuːnjətˈiːncˈaːɾ ʃˈuːnjəpˈa\u{303}ccʰˈʌh sˈaːtˈaːʈʰnˈɔː",
        "'e:kd'o:_! S'u:nj@-t'i:n_!c'a:r S'u:nj@-p'a~c_!c#'VH s'a:t'a:t.#_!n'O:",
    ),
    (
        "bn",
        "1234567",
        "ˈækdˈuj ʃˈunjɔtˈintʃʰˈaɾ pˈa\u{303}tʃtʃʰˈɔjʃˈat",
        "'&kd'uj_! S'unjOt'in_!tS#'ar p'a~tStS#'Oj_!S'at",
    ),
    (
        "ta",
        "1234567",
        "ˈonrʉˌiɹʌɳɖʉ sˈuɻijˌʌmmʉnrˌʉnaːnɡʉ ˈaɪndʉˌaːrʉʲˌeːɻʉ",
        "'onRU,irVn.d.U_! s'uz.ij,VmmU:nR,U_!na:ngU 'aIndU,a:RU_!;,e:z.U",
    ),
    (
        "mr",
        "1234567",
        "ˈeːkdˈoːnsvˌəlpəvɪɾˈaːm ʃˈuːnjətˈiːntʃˈaːrsvˌəlpəvɪɾˈaːm pˈaːcsˈʌhaːsˈaːt",
        "'e:kd'o:n_!sv,@lp@vI*'a:m S'u:nj@-t'i:n_!tS'a:rsv,@lp@vI*'a:m p'a:cs'VHa:_!s'a:t",
    ),
];

#[test]
fn digit_groups_take_separators_and_zero_padding() {
    if !std::path::Path::new("espeak-ng-data/en_dict").exists() {
        eprintln!("[SKIP] no data");
        return;
    }
    let bad: Vec<String> = CASES
        .iter()
        .flat_map(|&(l, d, i, x)| [(l, "--ipa", i, d), (l, "-x", x, d)])
        .filter_map(|(l, f, want, d)| {
            let text = format!("<speak><say-as interpret-as=\"tts:digits\">{d}</say-as></speak>");
            let got = run(l, f, &text);
            (got != want).then(|| format!("{l} {f} {d}: got {got:?}, C {want:?}"))
        })
        .collect();
    assert!(bad.is_empty(), "differs from C:\n{}", bad.join("\n"));
}
