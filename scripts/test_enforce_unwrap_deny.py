import unittest
import os
import tempfile
import enforce_unwrap_deny


class TestCheckLine(unittest.TestCase):
    """check_line() の単体テスト"""

    def test_unwrap_is_blocked(self):
        self.assertTrue(enforce_unwrap_deny.check_line("let v = result.unwrap();"))

    def test_expect_is_blocked(self):
        self.assertTrue(enforce_unwrap_deny.check_line('let v = result.expect("msg");'))

    def test_commented_line_is_allowed(self):
        self.assertFalse(enforce_unwrap_deny.check_line("// result.unwrap()"))

    def test_doc_comment_is_allowed(self):
        self.assertFalse(enforce_unwrap_deny.check_line("/// panics like .expect()"))

    def test_allow_unwrap_annotation(self):
        self.assertFalse(enforce_unwrap_deny.check_line("x.unwrap(); // allow-unwrap"))

    def test_allow_anti_pattern_annotation(self):
        self.assertFalse(
            enforce_unwrap_deny.check_line('.expect("valid") // allow-anti-pattern')
        )

    def test_unwrap_in_trailing_comment_is_not_violation(self):
        """コード部分に unwrap がなく、コメント部分にだけある場合は違反ではない"""
        self.assertFalse(enforce_unwrap_deny.check_line("let x = 1; // .unwrap()"))

    def test_safe_variants_are_not_blocked(self):
        self.assertFalse(enforce_unwrap_deny.check_line("let v = x.unwrap_or(0);"))
        self.assertFalse(enforce_unwrap_deny.check_line("let v = x.unwrap_or_default();"))
        self.assertFalse(
            enforce_unwrap_deny.check_line("let v = x.unwrap_or_else(|| 0);")
        )

    def test_clean_line_passes(self):
        self.assertFalse(enforce_unwrap_deny.check_line("let x = Some(1);"))

    def test_empty_line_passes(self):
        self.assertFalse(enforce_unwrap_deny.check_line(""))


class TestStripTrailingComment(unittest.TestCase):
    """_strip_trailing_comment() のエッジケーステスト"""

    def test_no_comment(self):
        self.assertEqual(
            enforce_unwrap_deny._strip_trailing_comment("let x = 1;"), "let x = 1;"
        )

    def test_simple_comment(self):
        self.assertEqual(
            enforce_unwrap_deny._strip_trailing_comment("let x = 1; // comment"), "let x = 1; "
        )

    def test_url_in_string_literal_preserved(self):
        """文字列リテラル内の // は切断しない（False Negative 防止の要）"""
        line = 'Regex::new("https://evil.com").unwrap();'
        result = enforce_unwrap_deny._strip_trailing_comment(line)
        self.assertIn(".unwrap()", result)

    def test_escaped_quote_in_string(self):
        line = r'let s = "escaped \" quote // inside".unwrap();'
        result = enforce_unwrap_deny._strip_trailing_comment(line)
        self.assertIn(".unwrap()", result)


class TestCheckFile(unittest.TestCase):
    """check_file() のファイルレベルテスト"""

    def _write_temp(self, content: str) -> str:
        f = tempfile.NamedTemporaryFile(mode="w", suffix=".rs", delete=False)
        f.write(content)
        f.close()
        return f.name

    def test_cfg_test_block_is_excluded(self):
        content = """\
fn prod() {
    let x = Some(1).unwrap(); // VIOLATION
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_ok() {
        let y = Some(2).unwrap(); // ALLOWED
    }
}
"""
        path = self._write_temp(content)
        try:
            violations = enforce_unwrap_deny.check_file(path)
            self.assertEqual(len(violations), 1)
            self.assertEqual(violations[0]["line_number"], 2)
        finally:
            os.remove(path)

    def test_url_in_string_literal_detected(self):
        """文字列リテラル内 // があっても unwrap は検出される"""
        content = 'let re = Regex::new("https://example.com").unwrap();\n'
        path = self._write_temp(content)
        try:
            violations = enforce_unwrap_deny.check_file(path)
            self.assertEqual(len(violations), 1)
        finally:
            os.remove(path)

    def test_braces_in_string_dont_corrupt_depth(self):
        """文字列リテラル内の波括弧がブロック追跡を壊さないことを確認"""
        content = """\
fn prod() {
    let s = "{ } { }";
    let x = Some(1).unwrap(); // VIOLATION
}

#[cfg(test)]
mod tests {
    fn test_ok() {
        let y = Some(2).unwrap(); // ALLOWED
    }
}
"""
        path = self._write_temp(content)
        try:
            violations = enforce_unwrap_deny.check_file(path)
            self.assertEqual(len(violations), 1, "Should detect exactly one violation in prod code")
            self.assertEqual(violations[0]["line_number"], 3)
        finally:
            os.remove(path)

    def test_violation_reason_distinguishes_macros(self):
        content = """\
let a = x.unwrap();
let b = y.expect("boom");
panic!("boom");
todo!("not yet");
unimplemented!("no");
unreachable!("impossible");
"""
        path = self._write_temp(content)
        try:
            violations = enforce_unwrap_deny.check_file(path)
            self.assertEqual(len(violations), 6)
            self.assertIn(".unwrap()", violations[0]["reason"])
            self.assertIn(".expect()", violations[1]["reason"])
            self.assertIn("panic!()", violations[2]["reason"])
            self.assertIn("todo!()", violations[3]["reason"])
            self.assertIn("unimplemented!()", violations[4]["reason"])
            self.assertIn("unreachable!()", violations[5]["reason"])
        finally:
            os.remove(path)

    def test_adjacent_line_annotation_is_allowed(self):
        """対象行の前または後にアノテーションがある場合も許可される"""
        content = """\
fn prod() {
    // allow-anti-pattern
    let x = Some(1).unwrap(); 
    
    let y = Some(2).unwrap();
    // allow-anti-pattern
    
    let z = Some(3).unwrap(); // VIOLATION
}
"""
        path = self._write_temp(content)
        try:
            violations = enforce_unwrap_deny.check_file(path)
            self.assertEqual(len(violations), 1, "Only z should be a violation")
            self.assertEqual(violations[0]["line_number"], 8)
        finally:
            os.remove(path)



class TestScanDirectory(unittest.TestCase):
    """scan_directory() のディレクトリ走査テスト"""

    def test_skips_test_files_by_name(self):
        with tempfile.TemporaryDirectory() as d:
            with open(os.path.join(d, "my_test.rs"), "w") as f:
                f.write("x.unwrap();")
            with open(os.path.join(d, "prod.rs"), "w") as f:
                f.write("y.unwrap();")

            violations = enforce_unwrap_deny.scan_directory(d)
            self.assertEqual(len(violations), 1)
            self.assertTrue(violations[0]["file"].endswith("prod.rs"))

    def test_skips_tests_directory(self):
        with tempfile.TemporaryDirectory() as d:
            test_dir = os.path.join(d, "tests")
            os.makedirs(test_dir)
            with open(os.path.join(test_dir, "integration.rs"), "w") as f:
                f.write("x.unwrap();")
            with open(os.path.join(d, "lib.rs"), "w") as f:
                f.write("y.unwrap();")

            violations = enforce_unwrap_deny.scan_directory(d)
            self.assertEqual(len(violations), 1)
            self.assertTrue(violations[0]["file"].endswith("lib.rs"))

    def test_non_rs_files_ignored(self):
        with tempfile.TemporaryDirectory() as d:
            with open(os.path.join(d, "readme.md"), "w") as f:
                f.write("x.unwrap();")

            violations = enforce_unwrap_deny.scan_directory(d)
            self.assertEqual(len(violations), 0)


class TestMain(unittest.TestCase):
    """main() の統合テスト"""

    def test_main_exits_0_on_clean_dir(self):
        with tempfile.TemporaryDirectory() as d:
            with open(os.path.join(d, "clean.rs"), "w") as f:
                f.write("let x = Some(1);\n")

            import unittest.mock
            with unittest.mock.patch("sys.argv", ["prog", d]):
                with self.assertRaises(SystemExit) as cm:
                    enforce_unwrap_deny.main()
                self.assertEqual(cm.exception.code, 0)

    def test_main_exits_1_on_violations(self):
        with tempfile.TemporaryDirectory() as d:
            with open(os.path.join(d, "bad.rs"), "w") as f:
                f.write("let x = Some(1).unwrap();\n")

            import unittest.mock
            with unittest.mock.patch("sys.argv", ["prog", d]):
                with self.assertRaises(SystemExit) as cm:
                    enforce_unwrap_deny.main()
                self.assertEqual(cm.exception.code, 1)

    def test_main_warns_on_invalid_directory(self):
        import unittest.mock
        import io
        with unittest.mock.patch("sys.argv", ["prog", "/nonexistent_dir_abc123"]):
            with unittest.mock.patch("sys.stderr", new_callable=io.StringIO) as mock_err:
                with self.assertRaises(SystemExit) as cm:
                    enforce_unwrap_deny.main()
                self.assertEqual(cm.exception.code, 0)
                self.assertIn("not a directory", mock_err.getvalue())

    def test_check_file_handles_unreadable_file(self):
        """読み取れないファイルは警告を出してスキップされる"""
        violations = enforce_unwrap_deny.check_file("/nonexistent/path/to/file.rs")
        self.assertEqual(len(violations), 0)


if __name__ == "__main__":
    unittest.main()
