// tests/spelled_word_phoneme_buffer.rs
//
// C `TranslateLetter` appends a letter's phonemes to the 200-byte word buffer only while the
// result stays under `N_WORD_PHONEMES`, so a spelled word that outgrows the buffer loses its
// tail.  Expected values are what C espeak-ng master (f0fb11a8) prints.

use std::process::Command;

fn run(lang: &str, flag: &str, text: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", flag, "-m", "-v", lang, text])
        .output()
        .expect("run");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn latin(count: usize) -> String {
    let letters: Vec<String> = (0..count).map(|i| char::from(b'a' + (i % 26) as u8).to_string()).collect();
    format!("{} done", letters.join(" . "))
}

fn check(lang: &str, text: &str, ipa: &str, x: &str) {
    assert_eq!(run(lang, "--ipa", text), ipa, "{lang} --ipa {text:?}");
    assert_eq!(run(lang, "-x", text), x, "{lang} -x {text:?}");
}

#[test]
fn eighty_dotted_letters_are_cut_at_the_buffer() {
    check(
        "en",
        &latin(80),
        "ˌeɪbˌiːsˈiːdˌiːˌiːˈɛfdʒˌiːˌeɪtʃˈaɪdʒˌeɪkˌeɪˈɛlˌɛmˌɛnˈəʊpˌiːkjˌuːˈɑːɹˌɛstˌiːjˈuːvˌiːdˌʌbəljˌuːˈɛkswˌaɪzˌɛdˈeɪbˌiːsˌiːdˈiːˌiːˌɛfdʒˈiːˌeɪtʃˌaɪdʒˈeɪkˌeɪˌɛlˈɛmˌɛnˌəʊpˈiːkjˌuːˌɑːɹˈɛstˌiːjˌuːvˌiːˈɛks dˈʌn",
        ",eIb,i:s'i:d,i:;,i:;'EfdZ,i:;,eItS'aIdZ,eIk,eI'El,Em,En'oUp,i:kj,u:'A@r,Est,i:j'u:v,i:d,Vb@Lj,u:'Eksw,aIz,Ed'eIb,i:s,i:d'i:;,i:;,EfdZ'i:;,eItS,aIdZ'eIk,eI,El'Em,En,oUp'i:kj,u:,A@r'Est,i:j,u:v,i:;'Eks d'Vn",
    );
}

#[test]
fn eighty_one_dotted_letters_are_cut_at_the_buffer() {
    check(
        "en",
        &latin(81),
        "ɐ dˈɒt bˌiːsˌiːdˈiːˌiːˌɛfdʒˈiːˌeɪtʃˌaɪdʒˈeɪkˌeɪˌɛlˈɛmˌɛnˌəʊpˈiːkjˌuːˌɑːɹˈɛstˌiːjˌuːvˈiːdˌʌbəljˌuːˌɛkswˈaɪzˌɛdˌeɪbˈiːsˌiːdˌiːˈiːˌɛfdʒˌiːˈeɪtʃˌaɪdʒˌeɪkˈeɪˌɛlˌɛmˈɛnˌəʊpˌiːkjˈuːˌɑːɹˌɛstˈiːjˌuːvˌiːdˈʌbəljˌuː dˈʌn",
        "a# d'0t b,i:s,i:d'i:;,i:;,EfdZ'i:;,eItS,aIdZ'eIk,eI,El'Em,En,oUp'i:kj,u:,A@r'Est,i:j,u:v'i:d,Vb@Lj,u:,Eksw'aIz,Ed,eIb'i:s,i:d,i:;'i:;,EfdZ,i:;'eItS,aIdZ,eIk'eI,El,Em'En,oUp,i:kj'u:,A@r,Est'i:j,u:v,i:d'Vb@Lj,u: d'Vn",
    );
}

#[test]
fn say_as_characters_are_cut_at_the_buffer() {
    let letters: String = ('a'..='z').cycle().take(60).collect();
    let text = format!("<speak><say-as interpret-as=\"characters\">{letters}</say-as> done</speak>");
    check(
        "en",
        &text,
        "ˌeɪbˌiːsˈiːdˌiːˌiːˈɛfdʒˌiːˌeɪtʃˈaɪdʒˌeɪkˌeɪˈɛlˌɛmˌɛnˈəʊpˌiːkjˌuːˈɑːˌɛstˌiːjˈuːvˌiːdˌʌbəljˌuːˈɛkswˌaɪzˌɛdˈeɪbˌiːsˌiːdˈiːˌiːˌɛfdʒˈiːˌeɪtʃˌaɪdʒˈeɪkˌeɪˌɛlˈɛmˌɛnˌəʊpˈiːkjˌuːˌɑːˈɛstˌiːjˌuːvˌiːˈɛks dˈʌn",
        ",eI_|b,i:_|s'i:_!d,i:_|,i:_|'Ef_!dZ,i:_|,eItS_|'aI_!dZ,eI_|k,eI_|'El_!,Em_|,En_|'oU_!p,i:_|kj,u:_|'A@_!,Es_|t,i:_|j'u:_!v,i:_|d,Vb@Lj,u:_|'Eks_!w,aI_|z,Ed_|'eI_!b,i:_|s,i:_|d'i:_!,i:_|,Ef_|dZ'i:_!,eItS_|,aI_|dZ'eI_!k,eI_|,El_|'Em_!,En_|,oU_|p'i:_!kj,u:_|,A@_|'Es_!t,i:_|j,u:_|v,i:_!'Eks_! d'Vn",
    );
}
