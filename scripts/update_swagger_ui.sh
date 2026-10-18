#!/bin/bash
# Copyright (c) 2026 Kirky.X🌠
# SPDX-License-Identifier: MIT
#
# 重新 vendor Swagger UI dist 功能子集到 assets/swagger-ui/。
#
# 用法:
#   scripts/update_swagger_ui.sh [version]        # 默认 5.32.6
#
# 离线/镜像场景用本地 zip 替代下载（内容仍受下方期望哈希 fail-closed 校验）:
#   SWAGGER_UI_ZIP=/path/to/v5.32.6.zip scripts/update_swagger_ui.sh 5.32.6
#
# 产出: assets/swagger-ui/ 下恰 10 个文件——dist 功能子集 8 项
# (index.html 引用链: index.html/index.css/swagger-initializer.js/swagger-ui.css/
# swagger-ui-bundle.js/swagger-ui-standalone-preset.js/favicon-32x32.png/
# favicon-16x16.png) + LICENSE + NOTICE.md。
#
# initializer 补丁(与 utoipa-swagger-ui build.rs 行为对齐, 保证运行期可注入
# openapi_url 且 UI 用默认 layout):
#   1. petstore 示例 url 行 → `url: __OPENAPI_URL__,` 占位符
#   2. 删除 `layout: "StandaloneLayout"` 行
# 任一模式失配即失败退出, 不产出半成品资产。
#
# 供应链锚点(EXPECTED_SHA256): 替换前对暂存资产逐文件 `sha256sum -c`
# fail-closed——下载源被改写/上游被攻陷时不会把篡改内容装进仓库。
# 升级版本时必须在本脚本同一提交显式更新期望哈希表并在 NOTICE.md 留档
# (哈希改动即 review 锚点)。

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RESET='\033[0m'

log_info() { echo -e "${BLUE}[INFO]${RESET} $1"; }
log_ok() { echo -e "${GREEN}[OK]${RESET} $1"; }
log_err() { echo -e "${RED}[ERROR]${RESET} $1" >&2; }

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="${1:-5.32.6}"
DEST="${PROJECT_ROOT}/assets/swagger-ui"
# 暂存目录与 DEST 同文件系统(assets/ 下), 校验通过后再做替换——不留部分拷贝
STAGE="${PROJECT_ROOT}/assets/.swagger-ui.stage.$$"
TAG_URL="https://github.com/swagger-api/swagger-ui/archive/refs/tags/v${VERSION}.zip"

# index.html 引用链决定的功能子集; es-bundle/standalone swagger-ui.js/oauth2-redirect
# 等 dist 文件不被引用, 不 vendor
DIST_FILES=(
  index.html
  index.css
  swagger-initializer.js
  swagger-ui.css
  swagger-ui-bundle.js
  swagger-ui-standalone-preset.js
  favicon-32x32.png
  favicon-16x16.png
)

# 期望哈希 = assets/swagger-ui/ 当前内容的唯一真值(8 dist 文件 + LICENSE)。
EXPECTED_SHA256=$(cat << 'EOF'
bb9928afd0ea8c12e124c42fef58fb080f36770389684badb2a4dcf548624eeb  index.html
9324807d424565a1639bb29f3754c8d4d45c1009c67674e996e33355f6929ce7  index.css
ccc8f9e52b90a67f0aa92570541fd9f000dbac5f939cc5fe236bd3dd11dc5cd1  swagger-initializer.js
ca238f7d7c2cf4480c1e77a9c3b9da915ab216e96ffd354e69076560c650c6de  swagger-ui.css
4b5a9b35a1adf37f00d39c0bae23cdb37007e2946240ef035137cc85b4d55349  swagger-ui-bundle.js
38f7334300f524e1d456b1c5f6ec45383316283c257444aa9453143fdb434c73  swagger-ui-standalone-preset.js
3ed612f41e050ca5e7000cad6f1cbe7e7da39f65fca99c02e99e6591056e5837  favicon-32x32.png
af24ad604dd7b3bcda8f975ab973075f4a2f70a4087944a12f8ef8b63a3e07c2  favicon-16x16.png
cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30  LICENSE
EOF
)

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "${TMP_DIR}" "${STAGE}"' EXIT

command -v unzip > /dev/null || { log_err "需要 unzip 解包"; exit 1; }
command -v python3 > /dev/null || { log_err "需要 python3 打 initializer 补丁"; exit 1; }
command -v sha256sum > /dev/null || { log_err "需要 sha256sum 做供应链校验"; exit 1; }

# ---- 1. 取 zip ----------------------------------------------------------
# zip 内顶层目录名必须等于 swagger-ui-${VERSION}(下方 SRC 存在性检查),
# 本地 zip 与参数版本不一致时会在此失败, NOTICE 不会记录错误版本
ZIP="${TMP_DIR}/swagger-ui.zip"
if [[ -n "${SWAGGER_UI_ZIP:-}" ]]; then
  [[ -f "${SWAGGER_UI_ZIP}" ]] || { log_err "SWAGGER_UI_ZIP 不存在: ${SWAGGER_UI_ZIP}"; exit 1; }
  cp "${SWAGGER_UI_ZIP}" "${ZIP}"
  log_info "使用本地 zip: ${SWAGGER_UI_ZIP}"
else
  command -v curl > /dev/null || { log_err "需要 curl 下载 tag zip"; exit 1; }
  log_info "下载 ${TAG_URL}"
  curl -fL --proto '=https' --tlsv1.2 --retry 3 -o "${ZIP}" "${TAG_URL}"
fi

unzip -q "${ZIP}" -d "${TMP_DIR}"
SRC="${TMP_DIR}/swagger-ui-${VERSION}"
[[ -d "${SRC}/dist" ]] || { log_err "zip 内未找到 swagger-ui-${VERSION}/dist"; exit 1; }

# ---- 2. 暂存 dist 子集 --------------------------------------------------
rm -rf "${STAGE}"
mkdir -p "${STAGE}"
for f in "${DIST_FILES[@]}"; do
  [[ -f "${SRC}/dist/${f}" ]] || { log_err "dist 缺少文件: ${f}"; exit 1; }
  cp "${SRC}/dist/${f}" "${STAGE}/${f}"
done
[[ -f "${SRC}/LICENSE" ]] || { log_err "zip 根缺少 LICENSE"; exit 1; }
cp "${SRC}/LICENSE" "${STAGE}/LICENSE"
[[ -f "${SRC}/NOTICE" ]] || { log_err "zip 根缺少 NOTICE(Apache-2.0 归属)"; exit 1; }

# ---- 3. initializer 补丁(失配即失败) ------------------------------------
# newline="\n": 固定 LF, 避免 Windows 上 CRLF 漂移破坏哈希锚点与 NOTICE 表
python3 - "${STAGE}/swagger-initializer.js" << 'PYEOF'
import sys

path = sys.argv[1]
src = open(path, encoding="utf-8").read()

OLD_URL = 'url: "https://petstore.swagger.io/v2/swagger.json",'
NEW_URL = "url: __OPENAPI_URL__,"
if src.count(OLD_URL) != 1:
    sys.exit(f"initializer 补丁失败: petstore url 行匹配 {src.count(OLD_URL)} 次(期望 1)")
src = src.replace(OLD_URL, NEW_URL)

LAYOUT_LINE = '\n    layout: "StandaloneLayout"'
if src.count(LAYOUT_LINE) != 1:
    sys.exit(f"initializer 补丁失败: layout 行匹配 {src.count(LAYOUT_LINE)} 次(期望 1)")
src = src.replace(LAYOUT_LINE, "")

open(path, "w", encoding="utf-8", newline="\n").write(src)
PYEOF

# ---- 4. 供应链校验(fail-closed, 替换前) ---------------------------------
if ! printf '%s\n' "${EXPECTED_SHA256}" | (cd "${STAGE}" && sha256sum -c -); then
  log_err "资产哈希与脚本期望值不一致——拒绝替换(下载源被改写或上游版本内容变化;"
  log_err "确认来源可信后, 在同一提交更新 EXPECTED_SHA256 并复核 NOTICE.md)"
  exit 1
fi
log_ok "期望哈希校验通过(9 文件)"

# ---- 5. NOTICE.md -------------------------------------------------------
{
  echo "# Swagger UI vendored assets"
  echo
  echo "- Version: v${VERSION}"
  echo "- Source: ${TAG_URL}"
  echo "- License: Apache-2.0 (see LICENSE)"
  echo "- Modifications: \`swagger-initializer.js\` 的 petstore 示例 url 行替换为"
  echo "  \`url: __OPENAPI_URL__,\` 占位符（运行期由 sdforge 注入实际 spec 地址），"
  echo "  并删除 \`layout: \"StandaloneLayout\"\` 行（UI 使用默认 layout）。"
  echo "  其余文件为上游 dist 原样拷贝。"
  echo "- Regenerate with: \`scripts/update_swagger_ui.sh ${VERSION}\`"
  echo
  echo "## Upstream NOTICE (Apache-2.0 §4(d))"
  echo
  echo "以下为上游仓库 NOTICE 原文，随再分发保留："
  echo
  echo '```'
  cat "${SRC}/NOTICE"
  echo '```'
  echo
  echo "## SHA-256"
  echo
  echo '```'
  (cd "${STAGE}" && sha256sum "${DIST_FILES[@]}" LICENSE)
  echo '```'
} > "${STAGE}/NOTICE.md"
# NOTICE.md 自身不参与哈希锚点(其内容含上面动态生成的表), 10 文件契约由下方计数保障

# ---- 6. 落盘前终检(全部校验通过后才替换, 不留半成品) --------------------
COUNT="$(ls -1 "${STAGE}" | wc -l)"
[[ "${COUNT}" -eq 10 ]] || { log_err "暂存资产文件数 ${COUNT} != 10"; exit 1; }
grep -q '__OPENAPI_URL__' "${STAGE}/swagger-initializer.js" || { log_err "占位符未注入"; exit 1; }
! grep -q 'layout: "StandaloneLayout"' "${STAGE}/swagger-initializer.js" || { log_err "layout 行未删除"; exit 1; }

# ---- 7. 落盘(同文件系统内暂存目录校验通过后替换) ------------------------
rm -rf "${DEST}"
mv "${STAGE}" "${DEST}"

log_ok "Swagger UI v${VERSION} 已 vendor 至 assets/swagger-ui/ (10 文件)"
(cd "${DEST}" && ls -la)
