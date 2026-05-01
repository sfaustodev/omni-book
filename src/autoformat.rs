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
