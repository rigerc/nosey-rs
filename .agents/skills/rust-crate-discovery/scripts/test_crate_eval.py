"""Offline regression checks for the Nosey evaluator policy."""
import unittest
from unittest.mock import patch
import urllib.error

from crate_eval import CrateSignals, compute_red_flags, fetch_rustsec, recommendation_for


class AdvisoryPolicyTests(unittest.TestCase):
    def test_every_grade_blocks_package_advisory_leads(self):
        sig = CrateSignals(advisories=[{"id": "RUSTSEC-2026-0001"}], rustsec_status="found")
        for grade in ("A", "B", "C", "D", "F"):
            self.assertTrue(recommendation_for(grade, compute_red_flags(sig)).startswith("BLOCK"))

    def test_unknown_and_skipped_are_not_clearance(self):
        for status in ("unknown", "not_checked"):
            sig = CrateSignals(rustsec_status=status)
            self.assertTrue(recommendation_for("A", compute_red_flags(sig)).startswith("CAUTION"))

    def test_failed_network_lookup_is_unknown(self):
        with patch("crate_eval.urllib.request.urlopen", side_effect=urllib.error.URLError("offline")):
            self.assertEqual(fetch_rustsec("rustix"), ([], "unknown"))


if __name__ == "__main__":
    unittest.main()
