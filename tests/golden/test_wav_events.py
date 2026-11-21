"""Self-test of the WAV and event ratchet logic.

    python3 -I tests/golden/test_wav_events.py
"""
import os
import sys
import unittest

sys.dont_write_bytecode = True
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
sys.argv = sys.argv[:1]

import wav_events as w  # noqa: E402


class Ratchet(unittest.TestCase):
    def test_exceeding_a_ceiling_fails(self):
        failed, lines = w.verdict({"en": {"wav": 3, "events": 1}}, {"en": {"wav": 2, "events": 1}})
        self.assertTrue(failed)
        self.assertIn("FAIL", lines[0])

    def test_exceeding_the_events_ceiling_fails(self):
        failed, _ = w.verdict({"en": {"wav": 0, "events": 5}}, {"en": {"wav": 0, "events": 4}})
        self.assertTrue(failed)

    def test_equal_counts_pass(self):
        failed, lines = w.verdict({"en": {"wav": 2, "events": 1}}, {"en": {"wav": 2, "events": 1}})
        self.assertFalse(failed)
        self.assertTrue(lines[0].endswith("OK"))

    def test_lower_counts_pass_and_ask_to_lower(self):
        failed, lines = w.verdict({"en": {"wav": 1, "events": 1}}, {"en": {"wav": 2, "events": 1}})
        self.assertFalse(failed)
        self.assertIn("LOWER THE CEILING", lines[0])

    def test_missing_ceiling_fails(self):
        failed, lines = w.verdict({"xx": {"wav": 0, "events": 0}}, {})
        self.assertTrue(failed)
        self.assertIn("NO CEILING", lines[0])

    def test_count_diffs_counts_each_kind_and_missing_rows(self):
        base = [["a", "1", "x", "s"], ["b", "2", "y", "s"], ["c", "3", "z", "s"]]
        rows = [["a", "1", "x"], ["b", "9", "y"]]
        self.assertEqual(w.count_diffs(base, rows), {"wav": 2, "events": 1})


class Parsing(unittest.TestCase):
    def test_rows_are_digested_and_missing_or_err_rows_are_err(self):
        out = b"running 1 test\n=== 0\nev 1 0 0 0 0 num=1\nsamples=5\n=== 1\nERR\n"
        got = w.parse_events(out, 3)
        self.assertEqual(got[0], w.digest(b"ev 1 0 0 0 0 num=1\nsamples=5\n"))
        self.assertEqual(got[1:], ["ERR", "ERR"])

    def test_noise_between_rows_is_ignored(self):
        a = w.parse_events(b"=== 0\nsamples=5\ntest dump ... ok\n", 1)
        self.assertEqual(a, [w.digest(b"samples=5\n")])


class Staleness(unittest.TestCase):
    def test_matching_text_is_fresh(self):
        self.assertFalse(w.is_stale([["a", "1", "x"]], [["a", "2", "y"]]))

    def test_changed_text_or_row_count_is_stale(self):
        self.assertTrue(w.is_stale([["a", "1", "x"]], [["b", "1", "x"]]))
        self.assertTrue(w.is_stale([["a", "1", "x"]], []))


class Breath(unittest.TestCase):
    def test_breath_voices_compare_by_length(self):
        self.assertEqual(w.wav_column(b"abc", True), "len3")
        self.assertEqual(w.wav_column(b"abc", True), w.wav_column(b"xyz", True))
        self.assertNotEqual(w.wav_column(b"abc", False), w.wav_column(b"xyz", False))

    def test_only_mn_among_the_corpora_has_breath(self):
        data = os.path.join(w.ROOT, "espeak-ng-data")
        self.assertEqual([x for x in w.langs() if w.has_breath(data, x)], ["mn"])


class Sampling(unittest.TestCase):
    def test_small_corpus_is_taken_whole(self):
        self.assertEqual(w.sample_rows(["a", "b"], 5), ["a", "b"])

    def test_sample_is_evenly_spaced_and_deterministic(self):
        lines = [str(i) for i in range(100)]
        self.assertEqual(w.sample_rows(lines, 4), ["0", "25", "50", "75"])
        self.assertEqual(w.sample_rows(lines, 4), w.sample_rows(lines, 4))

    def test_every_corpus_has_a_baseline_with_the_sampled_rows(self):
        for lang in w.langs():
            base = [r[0] for r in w.read_baseline(lang)]
            self.assertEqual(base, w.corpus_sample(lang), lang)


if __name__ == "__main__":
    unittest.main()
