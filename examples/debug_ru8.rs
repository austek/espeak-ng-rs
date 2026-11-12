use espeak_ng::dictionary::Dictionary;
use espeak_ng::translate::default_data_dir;
use std::path::PathBuf;

fn main() {
    let data_dir = PathBuf::from(default_data_dir());
    let dict = Dictionary::load("ru", &data_dir).unwrap();
    
    println!("letter_bits_offset = 0x{:x}", dict.letter_bits_offset);
    
    // Check if groups3 entries exist for each letter in "нет привет мама"
    for (ch, codepoint) in [
        ('н', 0x043Du32), ('е', 0x0435u32), ('т', 0x0442u32),
        ('п', 0x043Fu32), ('р', 0x0440u32), ('и', 0x0438u32),
        ('в', 0x0432u32), ('м', 0x043Cu32), ('а', 0x0430u32),
    ] {
        let lbo = dict.letter_bits_offset;
        if codepoint >= lbo {
            let idx = codepoint - lbo;
            let c2 = (idx + 1) as u8;
            let has_group = dict.group3(c2).is_some();
            println!("{ch} (U+{codepoint:04X}): idx={idx:3} c2={c2:3} group3_exists={has_group}");
        }
    }
    
    println!("\nGroups3 entries populated:");
    let count = dict.groups.groups3.iter().take(128).filter(|g| g.is_some()).count();
    println!("  count = {count}");

    println!("\nFirst 50 groups3 entries:");
    for (i, group) in dict.groups.groups3.iter().enumerate().take(50) {
        if let Some(off) = *group {
            println!("  groups3[{i}] = offset {off}");
        }
    }
}
