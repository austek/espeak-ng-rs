// Numbers with a colon, semicolon or comma between digits, decimal fractions and
// teens in languages that lack some number words. Values from upstream espeak-ng
// master (f0fb11a8), `espeak-ng -q --ipa -v <lang>` and `-q -x -v <lang>`.

fn cli(lang: &str, flag: &str, text: &str) -> String {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", flag, "-v", lang, "--", text])
        .env("ESPEAK_DATA_PATH", data)
        .output()
        .expect("run cli");
    String::from_utf8_lossy(&out.stdout).replace('\n', "|")
}

/// (voice, text, `--ipa` stdout, `-x` stdout)
const CASES: &[(&str, &str, &str, &str)] = &[
    ("lfn", "1,2", "ˈun virɡˈula dˈu|", "'un _viR2g'ula d'u|"),
    ("lfn", "12,34", "dˈesdˈu virɡˈula tɾˈe kwˈatɾo|", "d'esd'u _viR2g'ula t**'e kw'at**o|"),
    ("qdb", "1,2", "wˌæŋ koːmˈæʔ tuːʔ|", "w,aN ko:m'a? tu:?|"),
    ("an", "12,34", "dˈoθe kon tɾˈes kwˈatɾo|", "d'oTe kon t**'es kw'at**o|"),
    ("an", "17", "dˌeθisjˈɛt|", "d,eTisj'Et|"),
    ("ia", "12,34", "dˈetsedˌuo tres kwˈatro|", "d'etsed,uo tRes kw'atRo|"),
    ("jbo", "12", "pˈare|", "p'aRe_!|"),
    ("jbo", "12,34", "pˈare ʃˈivo|", "p'aRe_! S'ivo_!|"),
    ("jbo", "100", "panˈono|", "pan'o_!no|"),
    ("bs", "12:30", "dvˈanaɪzd dvˈotætʃkæ tɾˈidɛsɛt|", "dv'anaIzd dv'ot&tSk& tr**'idEsEt|"),
    ("sr", "9:05", "dˈɛvɛd dvˈotɐtʃkɐ nˈulɐ pˈɛt|", "d'EvEd dv'ot&tSk& n'ul& p'Et|"),
    ("hr", "1:2", "jˈedæn dvˈototʃkæ dvˈa|", "j'ed&n dv'ototSk& dv'a|"),
    ("uk", "1:2", "odˈen dβˈiː krˈapkɪ dβˈa|", "od'en dB'i: kR'apkI dB'a|"),
    ("fa", "1:2", "jˈek dˈonˈoq1teː dˈo|", "j'ek d'on'oq1te: d'o|"),
    ("ur", "1:2", "ˈeːk ɾˈaːbtˈah dˈoː|", "'e:k r'a:bt'ah d'o:|"),
    ("ky", "1:2", "bˈir qˈoS tSekˈit[ ekˈi|", "b'ir q'oS tSek'it ek'i|"),
    ("uk", "1;2", "odˈen krˈapka s kˈomoju dβˈa|", "od'en kR'apka s k'omoju dB'a|"),
    ("mk", "1:2", "ˈeden dvˈetotʃkˌi dvˈa|", "'eden dv'e_totSk,i dv'a|"),
    ("en", "12:30", "twˈɛlv θˈɜːti|", "tw'Elv T'3:ti|"),
    ("lt", "12:30", "dvʲˈiːɭika trʲˈɪsdʲeɕimt|", "dv;'i:l;ika t@-r;'Isd;eS;imt|"),
    ("lv", "12:30", "dˈiʋpadsmit trˈiːsdesmit|", "d'ivpadsmit tR'i:sdesmit|"),
    ("lij", "12:30", "dˈuzˑe e trˈeŋta|", "d'uze _e_ tR'eNta|"),
    ("mt", "34", "tlˈətiːnˌerba|", "tl'@ti:n,erba|"),
    ("mt", "3.5", "tlˌiːetˌa punt hˈəamsˌa|", "tl,i:et,a punt h'@ams,a|"),
    ("smj", "34", "nˌelljakˈolomolok|", "n,elljak'olomolok|"),
    ("ro", "12,34", "dˈoɪsprezˌetʃe vˈirɡulə trˈeɪzetʃʲʃipˈatru|", "d'oIsp@-*ez,etSe _v'irgul@ t@-*'eIzetSI^_Sip'at@-*u|"),
    ("ro", "1,05", "ˈunu vˈirɡulə zˈeɾo tʃˈintʃʲ|", "'unu _v'irgul@ z'e*o tS'intSI^|"),
    ("gn", "11", "pˌeteˈi\u{303}|", "p,ete'i~|"),
    ("lb", "17", "sˈiːvən|", "s'i:v@En|"),
    ("qya", "11", "mˈinɛ|", "m'inE|"),
];

#[test]
fn number_punctuation_matches_upstream() {
    for (lang, text, ipa, x) in CASES {
        assert_eq!(&cli(lang, "--ipa", text), ipa, "{lang} {text:?}");
        assert_eq!(&cli(lang, "-x", text), x, "{lang} {text:?}");
    }
}
