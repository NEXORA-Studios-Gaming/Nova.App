# Nova 文档站

基于 Astro 与 Starlight 的中文文档站，覆盖 Nova 用户指南、开发者指南和 API 参考。

## 本地开发

```sh
pnpm install
pnpm dev
```

默认开发地址为 `http://localhost:4321`。

## 构建

```sh
pnpm build
pnpm preview
```

文档内容位于 `src/content/docs/`。用户文档统一收录在 `user/`，开发者文档和 API 参考统一收录在 `developer/`；侧边栏会自动读取这两个目录。
