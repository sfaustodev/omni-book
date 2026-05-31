use omninote_core::autoformat::try_math_substitute;

#[test]
fn test_char_vs_byte_index_math_substitution_bug() {
    // We construct a first line with many multi-byte characters (e.g. '×') and a math expression,
    // followed by a second line with a simple math expression.
    // The first line has:
    // 20 instances of "3 × " where '×' is 2 bytes in UTF-8.
    // Plus "3 =\n"
    let mut content = String::new();
    for _ in 0..20 {
        content.push_str("3 × ");
    }
    content.push_str("3 =\n2 + 2 =");

    // Let's compute characters and bytes precisely:
    // Each "3 × " has 4 characters: '3', ' ', '×', ' '.
    // Each '×' is 2 bytes. So "3 × " is 5 bytes.
    // 20 * 4 = 80 characters, 20 * 5 = 100 bytes.
    // "3 =\n" is 4 characters ('3', ' ', '=', '\n'), 4 bytes.
    // So the first line ends at character index 84, byte index 104 (with '\n' at byte 103).
    // The second line is "2 + 2 =" (7 characters, 7 bytes).
    // Total characters: 84 + 7 = 91.
    // Total bytes: 104 + 7 = 111.

    // If the cursor is at the end of the second line:
    // Character index is 91.
    // Byte index is 111.
    let char_pos = 91;
    let byte_pos = 111;

    // 1. Bug Demonstration:
    // If we pass the character index (91) as if it were a byte index (which is what egui does because ccursor.index is a character index):
    // Inside current_line, search starts at 91. Since 91 is less than 103 (the byte index of the first line's newline),
    // it will never see the newline at 103 when decrementing!
    // It decrements start all the way to 0, returning start = 0.
    // It increments end up to 103 (the newline of the first line).
    // So it evaluates the first line ("3 × 3 × ... 3 =") instead of the second line ("2 + 2 =")!
    let res_buggy = try_math_substitute(&content, char_pos);
    assert!(
        res_buggy.is_some(),
        "Should evaluate the first line because of the index mismatch bug"
    );
    let (new_line, start, end) = res_buggy.unwrap();

    // This highlights the bug:
    // Instead of replacing the second line (start=104, end=111), it replaced the first line (start=0, end=103)!
    assert_eq!(start, 0);
    assert_eq!(end, 103);
    assert!(new_line.starts_with("3 × 3 ×"));

    // 2. Correct Behavior (when using byte-converted index):
    // If we pass the correct byte index (111):
    // It starts searching at 111, decrements and hits the newline at 103.
    // So start stops at 104, and end is 111.
    // It correctly evaluates the second line ("2 + 2 =")!
    let res_correct = try_math_substitute(&content, byte_pos);
    assert!(res_correct.is_some());
    let (new_line, start, end) = res_correct.unwrap();
    assert_eq!(start, 104);
    assert_eq!(end, 111);
    assert_eq!(new_line, "2 + 2 = 4");
}
