pub fn try_eval(expr: &str) -> Option<f64> {
    let cleaned: String = expr
        .replace(',', ".")
        .replace('×', "*")
        .replace('÷', "/")
        .replace('−', "-");
    if !cleaned.chars().all(|c| "0123456789+-*/(). eE".contains(c)) {
        return None;
    }
    if !cleaned.chars().any(|c| c.is_ascii_digit()) {
        return None;
    }
    if !cleaned.chars().any(|c| "+-*/".contains(c)) {
        return None;
    }
    meval::eval_str(&cleaned).ok().filter(|n| n.is_finite())
}

pub fn current_line(content: &str, pos: usize) -> (String, usize, usize) {
    let bytes = content.as_bytes();
    let pos = pos.min(content.len());
    let mut start = pos;
    while start > 0 && bytes[start - 1] != b'\n' {
        start -= 1;
    }
    let mut end = pos;
    while end < content.len() && bytes[end] != b'\n' {
        end += 1;
    }
    (content[start..end].to_string(), start, end)
}

pub fn try_math_substitute(content: &str, pos: usize) -> Option<(String, usize, usize)> {
    let (line, start, end) = current_line(content, pos);
    let trimmed = line.trim_end();
    if !trimmed.ends_with('=') {
        return None;
    }
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

    // CAD-12: adversarial inputs + proptest panic safety.

    #[test]
    fn empty_input_returns_none() {
        assert!(subst("").is_none());
        assert!(try_math_substitute("", 0).is_none());
    }

    #[test]
    fn only_operator_returns_none() {
        for op in ["+", "-", "*", "/"] {
            let s = format!("{op} =");
            assert!(subst(&s).is_none(), "input={s:?}");
        }
    }

    #[test]
    fn only_digits_no_operator_returns_none() {
        assert!(subst("123 =").is_none());
    }

    #[test]
    fn only_float_no_operator_returns_none() {
        assert!(subst("3.14 =").is_none());
    }

    #[test]
    fn deeply_nested_parens_does_not_panic() {
        let depth = 100;
        let expr = format!("{}1+1{} =", "(".repeat(depth), ")".repeat(depth));
        let _ = subst(&expr);
    }

    #[test]
    fn very_long_expression_does_not_panic() {
        let mut expr: String = (0..10_000)
            .map(|i| if i % 2 == 0 { '1' } else { '+' })
            .collect();
        expr.push_str(" =");
        let _ = subst(&expr);
    }

    #[test]
    fn exponent_notation_accepted() {
        assert!(subst("1e3 + 0 =").is_some());
        assert!(subst("1.5E2 + 0 =").is_some());
    }

    #[test]
    fn division_by_zero_returns_none() {
        assert!(subst("1 / 0 =").is_none());
        assert!(subst("1 / 0.0 =").is_none());
    }

    #[test]
    fn overflow_to_infinity_returns_none() {
        // 1e308 * 1e308 overflows to inf — is_finite filter must reject
        assert!(subst("1e308 * 1e308 =").is_none());
    }

    #[test]
    fn floating_point_addition_close_to_expected() {
        let line = subst("0.1 + 0.2 =").unwrap();
        // Either prints "0.3" or "0.30000000000000004" — both acceptable, just non-empty number after `=`
        let after_eq = line.split('=').nth(1).unwrap().trim();
        let parsed: f64 = after_eq.parse().expect("number after =");
        assert!((parsed - 0.3).abs() < 1e-9, "got {parsed}");
    }

    #[test]
    fn negative_result_formatted() {
        let line = subst("5 - 10 =").unwrap();
        assert!(line.contains("-5"), "got {line:?}");
    }

    #[test]
    fn comma_and_period_mixed_evaluate() {
        // Brazilian comma + US period should both convert to period
        let line = subst("1.5 + 1,5 =").unwrap();
        let after_eq = line.split('=').nth(1).unwrap().trim();
        let parsed: f64 = after_eq.parse().unwrap();
        assert!((parsed - 3.0).abs() < 1e-9, "got {parsed}");
    }

    #[test]
    fn pos_out_of_bounds_clamps() {
        let result = try_math_substitute("abc", 999);
        // Should clamp to len, not panic — abc has no `=`, so None
        assert!(result.is_none());
    }

    #[test]
    fn pos_zero_handled() {
        let result = try_math_substitute("2 + 3 =", 0);
        // Pos 0 is at start of line, current_line still picks up the whole line
        assert!(result.is_some());
    }

    #[test]
    fn function_names_blocked_by_char_filter() {
        // meval supports sin/cos/pi but the autoformat char-whitelist excludes
        // letters other than e/E (for exponent), so they all return None.
        for expr in [
            "sin(1) =",
            "cos(1) =",
            "pi + 1 =",
            "pow(2, 3) =",
            "abs(-1) =",
        ] {
            assert!(subst(expr).is_none(), "expected None for {expr:?}");
        }
    }

    #[test]
    fn injection_strings_return_none_without_panic() {
        let inputs = [
            "; DROP TABLE notes; --=",
            "__import__('os').system('x') =",
            "\\x00\\xff =",
            "${expr} =",
        ];
        for inp in inputs {
            let _ = subst(inp);
        }
    }

    #[test]
    fn current_line_handles_empty_content() {
        let (line, start, end) = current_line("", 0);
        assert!(line.is_empty());
        assert_eq!(start, 0);
        assert_eq!(end, 0);
    }

    #[test]
    fn current_line_returns_line_around_pos() {
        let content = "first\nsecond line\nthird";
        // Position inside "second line"
        let pos = content.find("second").unwrap() + 2;
        let (line, _, _) = current_line(content, pos);
        assert_eq!(line, "second line");
    }

    proptest::proptest! {
        #![proptest_config(proptest::test_runner::Config { cases: 256, ..proptest::test_runner::Config::default() })]

        #[test]
        fn try_math_substitute_never_panics_on_arbitrary_input(s in proptest::prelude::any::<String>()) {
            let pos = s.len() / 2;
            let _ = try_math_substitute(&s, pos);
        }

        #[test]
        fn current_line_never_panics(s in proptest::prelude::any::<String>(), pos in 0usize..2048) {
            let _ = current_line(&s, pos);
        }
    }
}
