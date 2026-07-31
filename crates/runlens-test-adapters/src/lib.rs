use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestSummary {
    pub passed: u32,
    pub failed: u32,
    pub skipped: u32,
    pub inconclusive: u32,
}

impl TestSummary {
    pub fn total(&self) -> u32 {
        self.passed + self.failed + self.skipped + self.inconclusive
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdapterKind {
    Auto,
    Pytest,
    Vitest,
    Junit,
    Gotest,
}

#[derive(Debug, thiserror::Error)]
pub enum AdapterError {
    #[error("junit adapter: {0}")]
    Junit(String),
}

#[derive(Debug)]
pub enum Adapter {
    Auto,
    Pytest(PytestAdapter),
    Vitest(VitestAdapter),
    Junit(JunitAdapter),
    Gotest(GotestAdapter),
}

impl Adapter {
    pub fn new(kind: AdapterKind) -> Self {
        match kind {
            AdapterKind::Auto => Self::Auto,
            AdapterKind::Pytest => Self::Pytest(PytestAdapter::default()),
            AdapterKind::Vitest => Self::Vitest(VitestAdapter::default()),
            AdapterKind::Junit => Self::Junit(JunitAdapter::default()),
            AdapterKind::Gotest => Self::Gotest(GotestAdapter::default()),
        }
    }

    pub fn feed(&mut self, chunk: &[u8]) -> Result<TestSummary, AdapterError> {
        let text = String::from_utf8_lossy(chunk);
        match self {
            Self::Auto => Ok(auto_feed(&text)),
            Self::Pytest(a) => Ok(a.consume(&text)),
            Self::Vitest(a) => Ok(a.consume(&text)),
            Self::Junit(a) => a.consume(&text),
            Self::Gotest(a) => Ok(a.consume(&text)),
        }
    }

    pub fn summary(&self) -> TestSummary {
        match self {
            Self::Auto => TestSummary::default(),
            Self::Pytest(a) => a.summary(),
            Self::Vitest(a) => a.summary(),
            Self::Junit(a) => a.summary(),
            Self::Gotest(a) => a.summary(),
        }
    }
}

fn auto_feed(text: &str) -> TestSummary {
    let mut s = TestSummary::default();
    let p = PytestAdapter::default().consume(text);
    let v = VitestAdapter::default().consume(text);
    let g = GotestAdapter::default().consume(text);
    s.passed = p.passed.max(v.passed).max(g.passed);
    s.failed = p.failed.max(v.failed).max(g.failed);
    s.skipped = p.skipped.max(v.skipped).max(g.skipped);
    s
}

#[derive(Debug, Default, Clone)]
pub struct PytestAdapter {
    last_summary: String,
}

impl PytestAdapter {
    pub fn consume(&mut self, text: &str) -> TestSummary {
        let mut s = TestSummary::default();
        self.consume_into(text, &mut s);
        s
    }

    pub fn summary(&self) -> TestSummary {
        let mut s = TestSummary::default();
        if let Some(parsed) = parse_pytest_line(&self.last_summary) {
            s.passed = parsed.passed;
            s.failed = parsed.failed;
            s.skipped = parsed.skipped;
            s.inconclusive = parsed.inconclusive;
        }
        s
    }

    fn consume_into(&mut self, text: &str, summary: &mut TestSummary) {
        for line in text.lines() {
            let l = line.trim_matches('=').trim();
            if l.contains(" in ") && (l.contains("passed") || l.contains("failed") || l.contains("error")) {
                self.last_summary = l.to_string();
            }
        }
        if let Some(parsed) = parse_pytest_line(&self.last_summary) {
            summary.passed = parsed.passed;
            summary.failed = parsed.failed;
            summary.skipped = parsed.skipped;
            summary.inconclusive = parsed.inconclusive;
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
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
    let keywords: &[(&str, &str)] = &[
        ("passed", "passed"),
        ("failed", "failed"),
        ("skipped", "skipped"),
        ("error", "error"),
    ];
    for (needle, target) in keywords {
        if let Some(pos) = l_lower.find(needle) {
            let before = &l[..pos];
            let iter = before
                .split(|c: char| !c.is_ascii_digit() && c != '-')
                .rev();
            for tok in iter {
                if let Ok(n) = tok.trim().parse::<i32>() {
                    if n >= 0 {
                        match *target {
                            "passed" => out.passed = n as u32,
                            "failed" => out.failed = n as u32,
                            "skipped" => out.skipped = n as u32,
                            "error" => out.inconclusive = n as u32,
                            _ => {}
                        }
                        any = true;
                    }
                    break;
                }
            }
        }
    }
    if any {
        Some(out)
    } else {
        None
    }
}

#[derive(Debug, Default)]
pub struct VitestAdapter {
    summary: TestSummary,
}

impl VitestAdapter {
    pub fn consume(&mut self, text: &str) -> TestSummary {
        for line in text.lines() {
            let l = line.trim();
            if l.contains('\u{2713}') {
                self.summary.passed += 1;
            } else if l.contains('\u{2717}') || l.contains('\u{00d7}') {
                self.summary.failed += 1;
            }
        }
        if let Some(tests_line) = text
            .lines()
            .find(|l| l.trim().starts_with("Tests:"))
        {
            let l = tests_line.to_lowercase();
            for (needle, target) in &[
                ("passed", "passed"),
                ("failed", "failed"),
                ("skipped", "skipped"),
            ] {
                if let Some(pos) = l.find(needle) {
                    let before = &l[..pos];
                    let iter = before
                        .split(|c: char| !c.is_ascii_digit())
                        .rev();
                    for tok in iter {
                        if let Ok(n) = tok.trim().parse::<u32>() {
                            match *target {
                                "passed" => self.summary.passed = n,
                                "failed" => self.summary.failed = n,
                                "skipped" => self.summary.skipped = n,
                                _ => {}
                            }
                            break;
                        }
                    }
                }
            }
        }
        self.summary
    }

    pub fn summary(&self) -> TestSummary {
        self.summary
    }
}

#[derive(Debug, Default)]
pub struct JunitAdapter {
    buffer: String,
    summary: TestSummary,
}

impl JunitAdapter {
    pub fn consume(&mut self, text: &str) -> Result<TestSummary, AdapterError> {
        self.buffer.push_str(text);
        let counts = scan_junit(&self.buffer)?;
        self.summary.passed = counts.passed;
        self.summary.failed = counts.failed;
        self.summary.skipped = counts.skipped;
        self.summary.inconclusive = counts.inconclusive;
        Ok(self.summary)
    }

    pub fn summary(&self) -> TestSummary {
        self.summary
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct JunitCounts {
    passed: u32,
    failed: u32,
    skipped: u32,
    inconclusive: u32,
}

fn scan_junit(buf: &str) -> Result<JunitCounts, AdapterError> {
    Ok(scan_junit_inner(buf))
}

fn scan_junit_inner(buf: &str) -> JunitCounts {
    let mut counts = JunitCounts::default();
    let mut failed = false;
    let mut skipped = false;
    let mut errored = false;
    let mut in_testcase = false;
    let mut i = 0usize;
    let bytes: Vec<char> = buf.chars().collect();
    while i < bytes.len() {
        if bytes[i] != '<' {
            i += 1;
            continue;
        }
        let mut j = i + 1;
        while j < bytes.len() && bytes[j] != '>' {
            j += 1;
        }
        if j >= bytes.len() {
            break;
        }
        let tag: String = bytes[i + 1..j].iter().collect();
        let tag = tag.trim();
        let (name, self_closing) = if let Some(stripped) = tag.strip_suffix('/') {
            (stripped.trim(), true)
        } else {
            (tag, false)
        };
        let closing = name.starts_with('/');
        let name = name.trim_start_matches('/');
        let name = name.split_whitespace().next().unwrap_or("");
        match name {
            "testcase" => {
                if !closing {
                    in_testcase = true;
                    failed = false;
                    skipped = false;
                    errored = false;
                } else if in_testcase {
                    if errored {
                        counts.inconclusive += 1;
                    } else if failed {
                        counts.failed += 1;
                    } else if skipped {
                        counts.skipped += 1;
                    } else {
                        counts.passed += 1;
                    }
                    in_testcase = false;
                }
            }
            "failure" => {
                if in_testcase && !closing {
                    failed = true;
                }
            }
            "error" => {
                if in_testcase && !closing {
                    errored = true;
                }
            }
            "skipped" => {
                if in_testcase && !closing {
                    skipped = true;
                }
            }
            _ => {}
        }
        if self_closing && name == "testcase" && in_testcase {
            if errored {
                counts.inconclusive += 1;
            } else if failed {
                counts.failed += 1;
            } else if skipped {
                counts.skipped += 1;
            } else {
                counts.passed += 1;
            }
            in_testcase = false;
        }
        i = j + 1;
    }
    counts
}

#[derive(Debug, Default)]
pub struct GotestAdapter {
    summary: TestSummary,
}

impl GotestAdapter {
    pub fn consume(&mut self, text: &str) -> TestSummary {
        for line in text.lines() {
            let l = line.trim();
            if l.starts_with("--- PASS: ") {
                self.summary.passed += 1;
            } else if l.starts_with("--- FAIL: ") {
                self.summary.failed += 1;
            } else if l.starts_with("--- SKIP: ") {
                self.summary.skipped += 1;
            } else if l.contains("\"Action\":\"pass\"") {
                self.summary.passed += 1;
            } else if l.contains("\"Action\":\"fail\"") {
                self.summary.failed += 1;
            } else if l.contains("\"Action\":\"skip\"") {
                self.summary.skipped += 1;
            }
        }
        self.summary
    }

    pub fn summary(&self) -> TestSummary {
        self.summary
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pytest_final_line_counts() {
        let mut a = Adapter::new(AdapterKind::Pytest);
        let s = a
            .feed(b"===\ntests/test_x.py .... 5 passed, 1 failed, 2 skipped in 0.42s\n===")
            .unwrap();
        assert_eq!(s.passed, 5);
        assert_eq!(s.failed, 1);
        assert_eq!(s.skipped, 2);
    }

    #[test]
    fn pytest_error_goes_to_inconclusive() {
        let mut a = Adapter::new(AdapterKind::Pytest);
        let s = a.feed(b"1 failed, 1 error in 0.11s").unwrap();
        assert_eq!(s.failed, 1);
        assert_eq!(s.inconclusive, 1);
    }

    #[test]
    fn vitest_counts_glyphs() {
        let mut a = Adapter::new(AdapterKind::Vitest);
        a.feed("✓ alpha\n".as_bytes()).unwrap();
        a.feed("✓ beta\n".as_bytes()).unwrap();
        a.feed("× gamma\n".as_bytes()).unwrap();
        let s = a.summary();
        assert_eq!(s.passed, 2);
        assert_eq!(s.failed, 1);
    }

    #[test]
    fn vitest_tests_block_overrides() {
        let mut a = Adapter::new(AdapterKind::Vitest);
        a.feed("Tests: 1 failed, 3 passed, 1 skipped (5)\n".as_bytes())
            .unwrap();
        let s = a.summary();
        assert_eq!(s.passed, 3);
        assert_eq!(s.failed, 1);
        assert_eq!(s.skipped, 1);
    }

    #[test]
    fn junit_full_document() {
        let mut a = Adapter::new(AdapterKind::Junit);
        let xml = r#"<?xml version="1.0"?>
<testsuite name="demo">
  <testcase name="a" classname="A"/>
  <testcase name="b" classname="A"><failure>boom</failure></testcase>
  <testcase name="c" classname="A"><skipped/></testcase>
  <testcase name="d" classname="A"><error>crash</error></testcase>
</testsuite>"#;
        let s = a.feed(xml.as_bytes()).unwrap();
        assert_eq!(s.passed, 1);
        assert_eq!(s.failed, 1);
        assert_eq!(s.skipped, 1);
        assert_eq!(s.inconclusive, 1);
    }

    #[test]
    fn junit_chunked_feeds() {
        let mut a = Adapter::new(AdapterKind::Junit);
        a.feed(b"<testsuite name=\"d\">").unwrap();
        let s = a
            .feed(b"<testcase name=\"a\"/><testcase name=\"b\"><failure>f</failure></testcase></testsuite>")
            .unwrap();
        assert_eq!(s.passed, 1);
        assert_eq!(s.failed, 1);
    }

    #[test]
    fn gotest_verbose_lines() {
        let mut a = Adapter::new(AdapterKind::Gotest);
        a.feed(b"--- PASS: TestFoo (0.00s)\n--- FAIL: TestBar (0.01s)\n--- SKIP: TestBaz (0.00s)\n")
            .unwrap();
        let s = a.summary();
        assert_eq!(s.passed, 1);
        assert_eq!(s.failed, 1);
        assert_eq!(s.skipped, 1);
    }

    #[test]
    fn gotest_json_action_lines() {
        let mut a = Adapter::new(AdapterKind::Gotest);
        a.feed(b"{\"Action\":\"pass\",\"Test\":\"TestFoo\"}\n{\"Action\":\"fail\",\"Test\":\"TestBar\"}\n")
            .unwrap();
        let s = a.summary();
        assert_eq!(s.passed, 1);
        assert_eq!(s.failed, 1);
    }

    #[test]
    fn total_aggregates() {
        let mut a = Adapter::new(AdapterKind::Vitest);
        a.feed(b"Tests: 1 failed, 3 passed (4)\n").unwrap();
        assert_eq!(a.summary().total(), 4);
    }

    #[test]
    fn empty_feed_is_harmless() {
        let mut a = Adapter::new(AdapterKind::Junit);
        let s = a.feed(b"").unwrap();
        assert_eq!(s.total(), 0);
    }
}
