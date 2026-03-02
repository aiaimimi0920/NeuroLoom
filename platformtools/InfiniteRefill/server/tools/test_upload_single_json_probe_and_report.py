#!/usr/bin/env python3
# -*- coding: utf-8 -*-

import json
import os
import tempfile
import unittest

from upload_single_json_probe_and_report import canonical_auth_json, load_auth


class UploadSingleJsonNormalizationTests(unittest.TestCase):
    def test_load_auth_accepts_utf8_bom(self):
        payload = {"type": "codex", "email": "a@b.com", "account_id": "acc_1"}
        content = "\ufeff" + json.dumps(payload, ensure_ascii=False)

        with tempfile.NamedTemporaryFile("w", delete=False, encoding="utf-8") as f:
            f.write(content)
            path = f.name

        try:
            got = load_auth(path)
            self.assertEqual(got.get("type"), "codex")
            self.assertEqual(got.get("email"), "a@b.com")
            self.assertEqual(got.get("account_id"), "acc_1")
        finally:
            os.remove(path)

    def test_canonical_auth_json_is_valid_and_no_bom(self):
        payload = {"b": 2, "a": 1, "nested": {"z": 3, "y": 2}}
        text = canonical_auth_json(payload)

        self.assertFalse(text.startswith("\ufeff"))
        parsed = json.loads(text)
        self.assertEqual(parsed["a"], 1)
        self.assertEqual(parsed["b"], 2)
        self.assertEqual(parsed["nested"]["y"], 2)


if __name__ == "__main__":
    unittest.main()
