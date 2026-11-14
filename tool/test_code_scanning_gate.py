import datetime
import json
import pathlib
import tempfile
import unittest
from contextlib import redirect_stdout
from io import StringIO

from code_scanning_gate import main

TODAY = datetime.date(2026, 11, 14)


def alert(tool, rule, url, severity="high"):
    return {
        "html_url": url,
        "tool": {"name": tool},
        "rule": {"id": rule, "security_severity_level": severity},
        "most_recent_instance": {"location": {"path": "no file associated with this alert"}},
    }


class CodeScanningGateTest(unittest.TestCase):
    def run_gate(self, alerts, ignores=""):
        with tempfile.TemporaryDirectory() as tmp:
            alerts_path = pathlib.Path(tmp) / "alerts.jsonl"
            alerts_path.write_text(
                "".join(json.dumps(a) + "\n" for a in alerts), encoding="utf-8"
            )
            ignores_path = pathlib.Path(tmp) / "ignores.toml"
            ignores_path.write_text(ignores, encoding="utf-8")
            out = StringIO()
            with redirect_stdout(out):
                code = main(["gate", str(alerts_path), str(ignores_path)], today=TODAY)
            return code, out.getvalue()

    def test_no_alert_passes(self):
        code, out = self.run_gate([])
        self.assertEqual(code, 0)
        self.assertIn("0 open alerts, 0 blocking", out)

    def test_an_open_alert_of_any_tool_and_severity_blocks(self):
        code, out = self.run_gate([alert("Scorecard", "FuzzingID", "u1", severity="low")])
        self.assertEqual(code, 1)
        self.assertIn("OPEN: Scorecard FuzzingID (low)", out)

    def test_an_accepted_alert_passes(self):
        ignores = (
            '[[IgnoredAlerts]]\ntool = "Scorecard"\nrule = "CodeReviewID"\n'
            'ignoreUntil = 2027-05-14\nreason = "single maintainer"\n'
        )
        code, out = self.run_gate([alert("Scorecard", "CodeReviewID", "u1")], ignores)
        self.assertEqual(code, 0)
        self.assertIn("accepted until 2027-05-14", out)

    def test_an_entry_only_accepts_its_own_tool(self):
        ignores = (
            '[[IgnoredAlerts]]\ntool = "CodeQL"\nrule = "CodeReviewID"\n'
            'ignoreUntil = 2027-05-14\nreason = "r"\n'
        )
        code, _ = self.run_gate([alert("Scorecard", "CodeReviewID", "u1")], ignores)
        self.assertEqual(code, 1)

    def test_a_passed_date_blocks(self):
        ignores = (
            '[[IgnoredAlerts]]\ntool = "Scorecard"\nrule = "CodeReviewID"\n'
            'ignoreUntil = 2026-11-13\nreason = "r"\n'
        )
        code, out = self.run_gate([alert("Scorecard", "CodeReviewID", "u1")], ignores)
        self.assertEqual(code, 1)
        self.assertIn("ignoreUntil 2026-11-13 has passed", out)

    def test_an_entry_without_reason_or_date_blocks(self):
        ignores = '[[IgnoredAlerts]]\ntool = "Scorecard"\nrule = "CodeReviewID"\n'
        code, out = self.run_gate([], ignores)
        self.assertEqual(code, 1)
        self.assertIn("no reason, no ignoreUntil date", out)

    def test_an_entry_without_an_alert_is_listed_only(self):
        ignores = (
            '[[IgnoredAlerts]]\ntool = "Scorecard"\nrule = "CodeReviewID"\n'
            'ignoreUntil = 2027-05-14\nreason = "r"\n'
        )
        code, out = self.run_gate([], ignores)
        self.assertEqual(code, 0)
        self.assertIn("the entry can go: Scorecard CodeReviewID", out)

    def test_the_same_alert_on_two_branches_counts_once(self):
        a = alert("CodeQL", "rust/x", "u1")
        code, out = self.run_gate([a, a])
        self.assertEqual(code, 1)
        self.assertIn("1 open alerts, 1 blocking", out)

    def test_unreadable_input_is_exit_2(self):
        with tempfile.TemporaryDirectory() as tmp:
            missing = str(pathlib.Path(tmp) / "missing")
            with redirect_stdout(StringIO()):
                self.assertEqual(main(["gate", missing, missing], today=TODAY), 2)


if __name__ == "__main__":
    unittest.main()
