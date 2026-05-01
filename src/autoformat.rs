pub fn try_eval(expr: &str) -> Option<f64> {
    let cleaned: String = expr
        .replace(',', ".")
        .replace('×', "*")
        .replace('÷', "/")
        .replace('−', "-");
    if !cleaned.chars().all(|c| "0123456789+-*/(). eE".contains(c)) {
        return None;
    }
    if !cleaned.chars().any(|c| c.is_ascii_digit()) { return None; }
    if !cleaned.chars().any(|c| "+-*/".contains(c)) { return None; }
    meval::eval_str(&cleaned).ok().filter(|n| n.is_finite())
}

pub fn current_line(content: &str, pos: usize) -> (String, usize, usize) {
    let bytes = content.as_bytes();
    let pos = pos.min(content.len());
    let mut start = pos;
    while start > 0 && bytes[start - 1] != b'\n' { start -= 1; }
    let mut end = pos;
    while end < content.len() && bytes[end] != b'\n' { end += 1; }
    (content[start..end].to_string(), start, end)
}

pub fn try_math_substitute(content: &str, pos: usize) -> Option<(String, usize, usize)> {
    let (line, start, end) = current_line(content, pos);
    let trimmed = line.trim_end();
    if !trimmed.ends_with('=') { return None; }
    let expr = trimmed.trim_end_matches('=').trim();
    let result = try_eval(expr)?;
    let formatted = if result.fract().abs() < 1e-10 {
        format!("{}", result as i64)
    } else {
        format!("{}", result)
    };
    Some((format!("{} {}", trimmed, formatted), start, end))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests exercise try_math_substitute (which exercises the evaluator internally)

    fn subst(content: &str) -> Option<String> {
        let pos = content.len();
        try_math_substitute(content, pos).map(|(line, _, _)| line)
    }

    #[test]
    fn addition_result() {
        let out = subst("2 + 3 =").unwrap();
        assert!(out.contains("5"), "Expected '5' in '{}'", out);
    }

    #[test]
    fn comma_decimal() {
        // Brazilian locale: 1,5 + 1,5 = 3
        let out = subst("1,5 + 1,5 =").unwrap();
        assert!(out.contains("3"), "Expected '3' in '{}'", out);
    }

    #[test]
    fn unicode_times() {
        // 3 × 4 = 12
        let out = subst("3 \u{00d7} 4 =").unwrap();
        assert!(out.contains("12"), "Expected '12' in '{}'", out);
    }

    #[test]
    fn unicode_div() {
        // 10 ÷ 2 = 5
        let out = subst("10 \u{00f7} 2 =").unwrap();
        assert!(out.contains("5"), "Expected '5' in '{}'", out);
    }

    #[test]
    fn no_trailing_equals_none() {
        assert!(subst("2 + 3").is_none());
    }

    #[test]
    fn non_numeric_none() {
        assert!(subst("hello + world =").is_none());
    }

    #[test]
    fn division_by_zero_none() {
        assert!(subst("1 / 0 =").is_none());
    }

    #[test]
    fn multiline_targets_current_line() {
        let content = "linha1\n2 + 2 =";
        let result = try_math_substitute(content, content.len());
        assert!(result.is_some());
        let (line, _, _) = result.unwrap();
        assert!(line.contains("4"), "Expected '4' in '{}'", line);
    }
}
