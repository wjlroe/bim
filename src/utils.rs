pub fn char_position_to_byte_position(input: &str, at: usize) -> usize {
    input.chars().take(at).map(|c| c.len_utf8()).sum()
}
