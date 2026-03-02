#!/usr/bin/env python3
# -*- coding: utf-8 -*-

import json
import unittest


def normalize_downloaded_json(raw: bytes) -> str:
    text = raw.decode("utf-8-sig", errors="strict")
    parsed = json.loads(text)
    return json.dumps(parsed, ensure_ascii=False, separators=(",", ":"), sort_keys=True) + "\n"


class DownloadJsonNormalizationTests(unittest.TestCase):
    def test_bom_json_is_normalized_without_bom(self):
        raw = ("\ufeff" + '{"b":2,"a":1}').encode("utf-8")
        out = normalize_downloaded_json(raw)
        self.assertFalse(out.startswith("\ufeff"))
        self.assertEqual(out, '{"a":1,"b":2}\n')

    def test_invalid_json_raises(self):
        raw = ("\ufeff" + "not-json").encode("utf-8")
        with self.assertRaises(json.JSONDecodeError):
            normalize_downloaded_json(raw)


if __name__ == "__main__":
    unittest.main()
