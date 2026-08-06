use std::collections::HashMap;

use crate::pty::TestSummary;

#[derive(Debug, Clone, Copy)]
pub enum TestAdapterHint {
    Auto,
    Junit,
    Pytest,
    Vitest,
    Gotest,
}

#[derive(Debug)]
pub enum AdapterState {
    Auto,
    Pytest(PytestAdapter),
    Vitest(VitestAdapter),
    Junit(JunitAdapter),
    Gotest(GotestAdapter),
}

pub fn detect_adapter(hint: TestAdapterHint) -> AdapterState {
    match hint {
        TestAdapterHint::Auto => AdapterState::Auto,
        TestAdapterHint::Pytest => AdapterState::Pytest(PytestAdapter::default()),
        TestAdapterHint::Vitest => AdapterState::Vitest(VitestAdapter::default()),
        TestAdapterHint::Junit => AdapterState::Junit(JunitAdapter::default()),
        TestAdapterHint::Gotest => AdapterState::Gotest(GotestAdapter::default()),
    }
}

pub fn run_adapter(adapter: &mut AdapterState, chunk: &[u8], summary: &mut TestSummary) {
    let text = String::from_utf8_lossy(chunk);
    match adapter {
        AdapterState::Auto => auto_adapt(&text, summary),
        AdapterState::Pytest(a) => a.consume(&text, summary),
        AdapterState::Vitest(a) => a.consume(&text, summary),
        AdapterState::Junit(a) => a.consume(&text, summary),
        AdapterState::Gotest(a) => a.consume(&text, summary),
    }
}

fn auto_adapt(text: &str, summary: &mut TestSummary) {
    if text.contains("PASS") || text.contains("FAIL") || text.contains("===") || text.contains("passed in") {
        PytestAdapter::default().consume(text, summary);
    } else if text.contains("✓") || text.contains("✗") || text.contains("Tests:") {
        VitestAdapter::default().consume(text, summary);
    } else if text.contains("PASS\t") || text.contains("FAIL\t") {
        GotestAdapter::default().consume(text, summary);
    } else if text.contains("testsuite") || text.contains("<testcase") {
        JunitAdapter::default().consume(text, summary);
    }
}

#[derive(Default, Debug)]
pub struct PytestAdapter {
    last_summary: String,
}

impl PytestAdapter {
    pub fn consume(&mut self, text: &str, summary: &mut TestSummary) {
        for line in text.lines() {
            let l = line.trim_matches('=').trim();
            if !l.contains(" in ") {
                continue;
            }
            if !(l.contains("passed") || l.contains("failed") || l.contains("error")) {
                continue;
            }
            self.last_summary = l.to_string();
        }
        if let Some(parsed) = parse_pytest_line(&self.last_summary) {
            summary.passed = parsed.passed;
            summary.failed = parsed.failed;
            summary.skipped = parsed.skipped;
            summary.inconclusive = parsed.inconclusive;
        }
    }
}

#[derive(Default, Debug, Clone, Copy)]
struct PytestCounts {
    passed: u32,
    failed: u32,
    skipped: u32,
    inconclusive: u32,
}

fn parse_pytest_line(line: &str) -> Option<PytestCounts> {
    let mut out = PytestCounts::default();
    let mut any = false;
    let l = line.trim_matches('=').trim();
    let l_lower = l.to_lowercase();
    let keywords: &[(&str, &str)] = &[("passed", "passed"), ("failed", "failed"), ("skipped", "skipped")];
    for (needle, target) in keywords {
        let Some(pos) = l_lower.find(needle) else {
            continue;
        };
        let before = &l[..pos];
        let trimmed = before.trim_end_matches(|c: char| !c.is_ascii_digit());
        let iter = trimmed.split(|c: char| !c.is_ascii_digit() && c != '-').rev();
        for tok in iter {
            let Ok(n) = tok.trim().parse::<i32>() else {
                continue;
            };
            if n >= 0 {
                match *target {
                    "passed" => out.passed = n as u32,
                    "failed" => out.failed = n as u32,
                    "skipped" => out.skipped = n as u32,
                    _ => {},
                }
                any = true;
            }
            break;
        }
    }
    if any {
        Some(out)
    } else {
        None
    }
}

#[derive(Default, Debug)]
pub struct VitestAdapter;

impl VitestAdapter {
    pub fn consume(&self, text: &str, summary: &mut TestSummary) {
        for line in text.lines() {
            if line.contains("✓") {
                summary.passed += 1;
            } else if line.contains("✗") || line.contains("FAIL") {
                summary.failed += 1;
            }
        }
    }
}

#[derive(Default, Debug)]
pub struct JunitAdapter {
    line_buf: String,
}

impl JunitAdapter {
    pub fn consume(&mut self, text: &str, summary: &mut TestSummary) {
        self.line_buf.push_str(text);
        let counts = scan_junit_buffer(&self.line_buf);
        summary.passed = counts.get("passed").copied().unwrap_or(0);
        summary.failed = counts.get("failed").copied().unwrap_or(0);
        summary.skipped = counts.get("skipped").copied().unwrap_or(0);
    }
}

fn scan_junit_buffer(buf: &str) -> HashMap<String, u32> {
    let mut cases = 0u32;
    let mut failed = 0u32;
    let mut errored = 0u32;
    let mut skipped = 0u32;
    for line in buf.lines() {
        if line.contains("<testcase") {
            cases += 1;
        }
        if line.contains("<failure>") || line.contains("<failure ") {
            failed += 1;
        }
        if line.contains("<error>") || line.contains("<error ") {
            errored += 1;
        }
        if line.contains("<skipped") {
            skipped += 1;
        }
    }
    let mut out = HashMap::new();
    let total_fail = failed + errored;
    out.insert("passed".into(), cases.saturating_sub(total_fail + skipped));
    out.insert("failed".into(), total_fail);
    out.insert("skipped".into(), skipped);
    out
}

#[derive(Default, Debug)]
pub struct GotestAdapter {
    line_buf: String,
}

impl GotestAdapter {
    pub fn consume(&mut self, text: &str, summary: &mut TestSummary) {
        self.line_buf.push_str(text);
        let mut passed = 0u32;
        let mut failed = 0u32;
        let mut skipped = 0u32;
        for line in self.line_buf.lines() {
            if line.starts_with("PASS\t") || line.contains("PASS:") {
                passed += 1;
            } else if line.starts_with("FAIL\t") || line.contains("FAIL:") {
                failed += 1;
            } else if line.contains("SKIP") {
                skipped += 1;
            }
        }
        summary.passed = passed;
        summary.failed = failed;
        summary.skipped = skipped;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pytest_passed_failed_summary() {
        let mut adapter = AdapterState::Pytest(PytestAdapter::default());
        let mut s = TestSummary::default();
        run_adapter(
            &mut adapter,
            b"===\ntests/test_x.py .... 5 passed, 1 failed in 0.42s\n===",
            &mut s,
        );
        assert_eq!(s.passed, 5);
        assert_eq!(s.failed, 1);
    }

    #[test]
    fn vitest_counts_glyphs() {
        let mut adapter = AdapterState::Vitest(VitestAdapter);
        let mut s = TestSummary::default();
        run_adapter(&mut adapter, "✓ alpha\n".as_bytes(), &mut s);
        run_adapter(&mut adapter, "✓ beta\n".as_bytes(), &mut s);
        run_adapter(&mut adapter, "FAIL gamma\n".as_bytes(), &mut s);
        assert_eq!(s.passed, 2);
        assert_eq!(s.failed, 1);
    }

    #[test]
    fn junit_handles_xml_chunks() {
        let mut adapter = AdapterState::Junit(JunitAdapter::default());
        let mut s = TestSummary::default();
        let xml = r#"<testsuite>
  <testcase name="a"/>
  <testcase name="b"><failure>msg</failure></testcase>
  <testcase name="c"><skipped/></testcase>
</testsuite>"#;
        run_adapter(&mut adapter, xml.as_bytes(), &mut s);
        assert_eq!(s.passed + s.failed + s.skipped, 3);
        assert_eq!(s.failed, 1);
        assert_eq!(s.skipped, 1);
        assert_eq!(s.passed, 1);
    }

    #[test]
    fn gotest_counts_pass_fail() {
        let mut adapter = AdapterState::Gotest(GotestAdapter::default());
        let mut s = TestSummary::default();
        run_adapter(&mut adapter, b"PASS\tTestFoo\nFAIL\tTestBar\nSKIP\tTestBaz\n", &mut s);
        assert_eq!(s.passed, 1);
        assert_eq!(s.failed, 1);
        assert_eq!(s.skipped, 1);
    }
}
