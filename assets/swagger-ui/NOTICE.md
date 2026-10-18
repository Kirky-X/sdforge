# Swagger UI vendored assets

- Version: v5.32.6
- Source: https://github.com/swagger-api/swagger-ui/archive/refs/tags/v5.32.6.zip
- License: Apache-2.0 (see LICENSE)
- Modifications: `swagger-initializer.js` 的 petstore 示例 url 行替换为
  `url: __OPENAPI_URL__,` 占位符（运行期由 sdforge 注入实际 spec 地址），
  并删除 `layout: "StandaloneLayout"` 行（UI 使用默认 layout）。
  其余文件为上游 dist 原样拷贝。
- Regenerate with: `scripts/update_swagger_ui.sh 5.32.6`

## Upstream NOTICE (Apache-2.0 §4(d))

以下为上游仓库 NOTICE 原文，随再分发保留：

```
swagger-ui
Copyright 2020-2021 SmartBear Software Inc.
```

## SHA-256

```
bb9928afd0ea8c12e124c42fef58fb080f36770389684badb2a4dcf548624eeb  index.html
9324807d424565a1639bb29f3754c8d4d45c1009c67674e996e33355f6929ce7  index.css
ccc8f9e52b90a67f0aa92570541fd9f000dbac5f939cc5fe236bd3dd11dc5cd1  swagger-initializer.js
ca238f7d7c2cf4480c1e77a9c3b9da915ab216e96ffd354e69076560c650c6de  swagger-ui.css
4b5a9b35a1adf37f00d39c0bae23cdb37007e2946240ef035137cc85b4d55349  swagger-ui-bundle.js
38f7334300f524e1d456b1c5f6ec45383316283c257444aa9453143fdb434c73  swagger-ui-standalone-preset.js
3ed612f41e050ca5e7000cad6f1cbe7e7da39f65fca99c02e99e6591056e5837  favicon-32x32.png
af24ad604dd7b3bcda8f975ab973075f4a2f70a4087944a12f8ef8b63a3e07c2  favicon-16x16.png
cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30  LICENSE
```
