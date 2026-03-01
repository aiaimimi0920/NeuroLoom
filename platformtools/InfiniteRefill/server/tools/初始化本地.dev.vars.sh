#!/usr/bin/env sh
set -eu

# 说明：
# - 如果 server/.dev.vars 不存在，则从 server/.dev.vars.example 复制生成。
# - 该文件被 gitignore 忽略，你可以在其中填写真实 secrets。

EXAMPLE="server/.dev.vars.example"
TARGET="server/.dev.vars"

if [ -f "$TARGET" ]; then
  echo "[OK] 已存在：$TARGET"
  echo "[INFO] 如需重置，请手动删除 $TARGET 后重新运行本脚本。"
  exit 0
fi

if [ ! -f "$EXAMPLE" ]; then
  echo "[ERROR] 找不到模板文件：$EXAMPLE" >&2
  exit 1
fi

cp "$EXAMPLE" "$TARGET"

echo "[OK] 已生成：$TARGET"
echo "[NEXT] 请打开 $TARGET 填写真实密钥（ADMIN_TOKEN/R2_* 等）。"
