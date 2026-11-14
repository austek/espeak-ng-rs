// The 32-bit clause terminator of C espeak-ng master f0fb11a8 (translate.h:210-241) and the
// clause properties of ucd-tools proplist.c that ReadClause turns into terminators.

use espeak_ng::translate::readclause::Terminator as T;

#[test]
fn constants_match_translate_h() {
    assert_eq!(T::PARAGRAPH.bits(), 0x80046);
    assert_eq!(T::EOF.bits(), 0x90028);
    assert_eq!(T::VOICE.bits(), 0x24000);
    assert_eq!(T::PERIOD.bits(), 0x80028);
    assert_eq!(T::COMMA.bits(), 0x41014);
    assert_eq!(T::EXCLAMATION.bits(), 0x8302d);
    assert_eq!(T::COLON.bits(), 0x4001e);
    assert_eq!(T::NONE.bits(), 0x4000);
    assert_eq!(T::QUESTION.bits(), 0x82028);
    assert_eq!(T::SEMICOLON.bits(), 0x4101e);
    assert_eq!(T::SHORTCOMMA.bits(), 0x41004);
    assert_eq!(T::SHORTFALL.bits(), 0x40004);
}

#[test]
fn pause_applies_the_long_flag_times_32() {
    assert_eq!(T::PERIOD.pause_ms(), 400);
    assert_eq!(T::EXCLAMATION.pause_ms(), 450);
    assert_eq!(T::NONE.plus_pause(50).pause_ms(), 500);
    assert_eq!(T::NONE.plus_pause(50).with(T::PAUSE_LONG).pause_ms(), 16000);
}

#[test]
fn accessors_read_the_type_bits() {
    assert!(T::PERIOD.is_sentence() && !T::COMMA.is_sentence());
    assert!(T::EOF.is_eof() && !T::PERIOD.is_eof());
    assert!(T::VOICE.is_voice_change());
    assert_eq!(T::QUESTION.intonation(), 2);
    assert_eq!(T::NONE.intonation(), 4);
    assert!(T::PERIOD.with(T::DOT_AFTER_LAST_WORD).dot_after_last_word());
}

#[test]
fn codepoints_map_to_terminators_with_flags() {
    assert_eq!(T::from_codepoint('.' as u32), Some(T::PERIOD));
    assert_eq!(T::from_codepoint('?' as u32), Some(T::QUESTION));
    assert_eq!(T::from_codepoint('!' as u32), Some(T::EXCLAMATION));
    assert_eq!(T::from_codepoint(',' as u32), Some(T::COMMA));
    assert_eq!(T::from_codepoint(':' as u32), Some(T::COLON));
    assert_eq!(T::from_codepoint(';' as u32), Some(T::SEMICOLON));
    assert_eq!(T::from_codepoint('a' as u32), None);
    assert_eq!(T::from_codepoint(0x2029), Some(T::PARAGRAPH));
}

#[test]
fn special_properties_set_their_flags() {
    let opt = T::OPTIONAL_SPACE_AFTER;
    assert_eq!(T::from_codepoint(0x3002), Some(T::PERIOD.with(opt)));
    assert_eq!(T::from_codepoint(0xbf), Some(T::SEMICOLON.with(opt)));
    assert_eq!(T::from_codepoint(0xa1), Some(T::SEMICOLON.with(opt)));
    assert_eq!(T::from_codepoint(0x2026), Some(T::SEMICOLON.with(T::SPEAK_PUNCTUATION_NAME).with(opt)));
    assert_eq!(T::from_codepoint(0x2014), Some(T::SEMICOLON));
    assert_eq!(T::from_codepoint(0x55e), Some(T::QUESTION.with(T::PUNCTUATION_IN_WORD)));
    assert_eq!(T::from_codepoint(0x2048), None);
}
