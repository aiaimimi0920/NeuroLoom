#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""批量上传某个文件夹下所有 auth json 的“身份信息 + 健康探测结果”到无限续杯服务器（合规版/模式A）。

- 默认：对每个 json 做本地 wham 探测，然后上报到服务端
- 可选：只注册身份（不探测）

示例：
  python tools/upload_folder_json_probe_and_report.py \
    --server http://127.0.0.1:8787 \
    --upload-key YOUR_UPLOAD_KEY \
    --dir C:\Users\Administrator\.cli-proxy-api \
    --glob "*.json" \
    --mode probe
"""

from __future__ import annotations

import argparse
import asyncio
import glob
import os
from typing import List

from upload_single_json_probe_and_report import main as upload_one_main


def expand_files(folder: str, pattern: str) -> List[str]:
    folder = os.path.abspath(folder)
    p = os.path.join(folder, pattern)
    files = glob.glob(p)
    files = [f for f in files if os.path.isfile(f)]
    files.sort()
    return files


def main() -> int:
    p = argparse.ArgumentParser(description="Upload folder auth json -> report to refill server (compliant mode A)")
    p.add_argument("--server", required=True)
    p.add_argument("--upload-key", required=True)
    p.add_argument("--dir", required=True)
    p.add_argument("--glob", default="*.json")
    p.add_argument("--mode", choices=["probe", "register"], default="probe")
    p.add_argument("--timeout", type=int, default=15)
    args = p.parse_args()

    files = expand_files(args.dir, args.glob)
    if not files:
        print("[SKIP] no files")
        return 0

    # 串行（更稳）：避免对第三方接口产生过多并发。
    ok = 0
    bad = 0

    for f in files:
        argv = [
            "--server",
            args.server,
            "--upload-key",
            args.upload_key,
            "--file",
            f,
            "--mode",
            args.mode,
            "--timeout",
            str(args.timeout),
        ]
        try:
            import sys

            old = sys.argv
            sys.argv = ["upload_single_json_probe_and_report.py"] + argv
            code = upload_one_main()
            if code == 0:
                ok += 1
            else:
                bad += 1
        finally:
            sys.argv = old

    print(f"done ok={ok} bad={bad} total={len(files)}")
    return 0 if bad == 0 else 2


if __name__ == "__main__":
    raise SystemExit(main())
