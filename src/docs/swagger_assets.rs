// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Swagger UI 自 vendor 静态资产（`assets/swagger-ui/`，版本与修改见其 `NOTICE.md`）。
//!
//! dist 功能子集编译期内嵌，共 8 项——`index.html` 的引用链决定最小集：
//! index.html / index.css / swagger-ui.css / swagger-ui-bundle.js /
//! swagger-ui-standalone-preset.js / favicon-32x32.png / favicon-16x16.png
//! 为纯静态文件；swagger-initializer.js 含 `__OPENAPI_URL__` 占位符，由
//! [`render_initializer`] 在请求期注入 spec 地址。
//!
//! Content-Type 取值与 utoipa-swagger-ui 时期的 mime_guess 输出对齐（迁移
//! 保真）。资产生成/升级一律经 `scripts/update_swagger_ui.sh`。

/// 静态文件表：(路径, 内容, Content-Type)。不含 initializer（见模块注释）。
static ASSETS: &[(&str, &[u8], &str)] = &[
    (
        "index.html",
        include_bytes!("../../assets/swagger-ui/index.html"),
        "text/html",
    ),
    (
        "index.css",
        include_bytes!("../../assets/swagger-ui/index.css"),
        "text/css",
    ),
    (
        "swagger-ui.css",
        include_bytes!("../../assets/swagger-ui/swagger-ui.css"),
        "text/css",
    ),
    (
        "swagger-ui-bundle.js",
        include_bytes!("../../assets/swagger-ui/swagger-ui-bundle.js"),
        "text/javascript",
    ),
    (
        "swagger-ui-standalone-preset.js",
        include_bytes!("../../assets/swagger-ui/swagger-ui-standalone-preset.js"),
        "text/javascript",
    ),
    (
        "favicon-32x32.png",
        include_bytes!("../../assets/swagger-ui/favicon-32x32.png"),
        "image/png",
    ),
    (
        "favicon-16x16.png",
        include_bytes!("../../assets/swagger-ui/favicon-16x16.png"),
        "image/png",
    ),
];

/// initializer 模板（含 `__OPENAPI_URL__` 占位符）。
const INITIALIZER_TEMPLATE: &str = include_str!("../../assets/swagger-ui/swagger-initializer.js");

/// initializer 的对外路径（handler 特判路由到 [`render_initializer`]）。
pub(super) const INITIALIZER_PATH: &str = "swagger-initializer.js";

/// initializer 的 Content-Type。
pub(super) const INITIALIZER_CONTENT_TYPE: &str = "text/javascript";

/// 查静态文件表；空 tail 与 `/` 归一化为 index.html。
pub(super) fn get(path: &str) -> Option<(&'static [u8], &'static str)> {
    let path = if path.is_empty() || path == "/" {
        "index.html"
    } else {
        path
    };
    ASSETS
        .iter()
        .find(|(name, _, _)| *name == path)
        .map(|(_, bytes, content_type)| (*bytes, *content_type))
}

/// 渲染 initializer：`__OPENAPI_URL__` 替换为 JSON 字符串字面量。
///
/// 经 serde_json 转义（引号/反斜杠安全），调用方传入任意 spec 地址均可安全注入。
pub(super) fn render_initializer(openapi_url: &str) -> Vec<u8> {
    // String → JSON 字符串的序列化不会失败（无自定义 Serialize 参与）。
    let literal = serde_json::to_string(openapi_url)
        .expect("serializing a &str to a JSON string cannot fail");
    INITIALIZER_TEMPLATE
        .replace("__OPENAPI_URL__", &literal)
        .into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    const STATIC_PATHS: &[(&str, &str)] = &[
        ("index.html", "text/html"),
        ("index.css", "text/css"),
        ("swagger-ui.css", "text/css"),
        ("swagger-ui-bundle.js", "text/javascript"),
        ("swagger-ui-standalone-preset.js", "text/javascript"),
        ("favicon-32x32.png", "image/png"),
        ("favicon-16x16.png", "image/png"),
    ];

    #[test]
    fn all_vendored_static_assets_resolve_with_expected_content_type() {
        for (path, content_type) in STATIC_PATHS {
            let (bytes, ct) =
                get(path).unwrap_or_else(|| panic!("{path} must resolve in the asset table"));
            assert!(!bytes.is_empty(), "{path} must not be empty");
            assert_eq!(ct, *content_type, "{path} content type");
        }
    }

    #[test]
    fn empty_tail_and_slash_resolve_to_index_html() {
        let (root, root_ct) = get("").expect("empty tail resolves");
        let (slash, slash_ct) = get("/").expect("slash tail resolves");
        let (index, index_ct) = get("index.html").expect("index.html resolves");
        assert_eq!(root, index);
        assert_eq!(slash, index);
        assert_eq!(root_ct, "text/html");
        assert_eq!(slash_ct, "text/html");
        assert_eq!(index_ct, "text/html");
    }

    #[test]
    fn unknown_and_initializer_paths_are_not_in_static_table() {
        assert!(get("no-such.xyz").is_none());
        assert!(
            get(INITIALIZER_PATH).is_none(),
            "initializer 走 render_initializer，不进静态表"
        );
    }

    #[test]
    fn rendered_initializer_injects_url_and_has_no_layout() {
        let js = String::from_utf8(render_initializer("/host/spec.json")).expect("utf-8");
        assert!(
            js.contains("url: \"/host/spec.json\""),
            "injected url: {js}"
        );
        assert!(
            !js.contains("__OPENAPI_URL__"),
            "placeholder consumed: {js}"
        );
        assert!(!js.contains("layout:"), "layout line stays removed: {js}");
        assert!(js.contains("dom_id: '#swagger-ui'"), "dom_id kept: {js}");
        assert!(js.contains("deepLinking: true"), "deepLinking kept: {js}");
        assert!(
            js.contains("SwaggerUIStandalonePreset"),
            "standalone preset kept: {js}"
        );
    }

    #[test]
    fn rendered_initializer_escapes_quotes_and_backslashes_in_url() {
        let js = String::from_utf8(render_initializer("/a\"b\\c")).expect("utf-8");
        assert!(js.contains(r#"url: "/a\"b\\c""#), "escaped literal: {js}");
        assert!(
            !js.contains("__OPENAPI_URL__"),
            "placeholder consumed: {js}"
        );
    }

    /// 磁盘资产目录与内嵌清单一致：8 dist 文件 + LICENSE + NOTICE.md，
    /// 防止升级（scripts/update_swagger_ui.sh）与代码清单漂移。
    #[test]
    fn vendored_assets_directory_matches_embedded_inventory() {
        let dir = assets_dir();
        let mut names: Vec<String> = std::fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
            .map(|entry| {
                entry
                    .expect("directory entry readable")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        names.sort();
        let mut expected: Vec<String> = STATIC_PATHS
            .iter()
            .map(|(path, _)| (*path).to_string())
            .chain([
                INITIALIZER_PATH.to_string(),
                "LICENSE".to_string(),
                "NOTICE.md".to_string(),
            ])
            .collect();
        expected.sort();
        assert_eq!(
            names, expected,
            "vendored asset directory drifted from the embedded inventory"
        );
    }

    /// NOTICE.md 自证：其 SHA-256 表必须与目录内文件字节逐项一致。
    ///
    /// 哈希唯一来源为 NOTICE.md（由 scripts/update_swagger_ui.sh 生成）——
    /// 单边修改文件或表值即测试红；表格式（`## SHA-256` 段 + 围栏代码块 +
    /// `sha256sum` 行）是本测试固定的解析契约，脚本升级时不得改变。
    #[test]
    fn notice_sha256_table_matches_vendored_bytes() {
        use sha2::{Digest, Sha256};

        let dir = assets_dir();
        let notice = std::fs::read_to_string(dir.join("NOTICE.md")).expect("NOTICE.md readable");
        let section = notice
            .split_once("## SHA-256")
            .expect("NOTICE.md must contain a '## SHA-256' section")
            .1;
        let block = section
            .split_once("```")
            .expect("SHA-256 section must open a fenced block")
            .1
            .split_once("```")
            .expect("SHA-256 section must close the fenced block")
            .0;

        let table: Vec<(String, String)> = block
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| {
                let mut columns = line.split_whitespace();
                let digest = columns.next().expect("hash column").to_string();
                let name = columns.next().expect("file column").to_string();
                assert!(columns.next().is_none(), "unexpected extra column: {line}");
                assert_eq!(
                    digest.len(),
                    64,
                    "not a SHA-256 hex digest in NOTICE.md: {line}"
                );
                (name, digest)
            })
            .collect();

        let mut names: Vec<&str> = table.iter().map(|(name, _)| name.as_str()).collect();
        names.sort_unstable();
        let mut expected: Vec<&str> = STATIC_PATHS
            .iter()
            .map(|(path, _)| *path)
            .chain([INITIALIZER_PATH, "LICENSE"])
            .collect();
        expected.sort_unstable();
        assert_eq!(
            names, expected,
            "NOTICE SHA-256 table must cover exactly the 9-file contract"
        );

        for (name, digest) in &table {
            let bytes =
                std::fs::read(dir.join(name)).unwrap_or_else(|e| panic!("read {name}: {e}"));
            let computed: String = Sha256::digest(&bytes)
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect();
            assert_eq!(
                &computed, digest,
                "{name} bytes drifted from the NOTICE.md SHA-256 table"
            );
        }
    }

    fn assets_dir() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/swagger-ui")
    }
}
