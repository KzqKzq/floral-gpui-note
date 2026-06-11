# Floral GPUI Notepad

一个用 **Rust + GPUI + gpui-component** 搭出来的原生 Markdown 记事本起步工程。目标是参考 Floral Notepaper 的「纸张感 / 左侧信息区 / 编辑与预览」布局，但不使用 Tauri、React 或 WebView。

## 已实现的 MVP 功能

- 原生 GPUI 窗口，不嵌入浏览器 WebView
- Markdown 编辑区，基于 `gpui-component::input::InputState` 的 code editor 模式
- Markdown 预览区，基于 `gpui-component::text::TextView::markdown`
- 编辑 / 分栏 / 预览三种模式
- 新建、打开、保存、另存为 `.md`
- 最近打开文件展示
- 暖色纸张风格主题变量集中在 `src/theme.rs`

## 目录结构

```txt
floral-gpui-notepad/
├── Cargo.toml
├── README.md
├── docs/
│   └── ROADMAP.md
└── src/
    ├── main.rs      # GPUI 启动、assets、Root 初始化
    ├── app.rs       # 主窗口状态、布局、编辑/预览/文件操作
    ├── fs.rs        # 文件选择、读写 markdown
    └── theme.rs     # 视觉主题色
```

## 运行

```bash
cargo run
```

Release 构建：

```bash
cargo run --release
```

## 环境要求

`gpui-component` 文档当前要求 Rust 1.90+。Windows、macOS、Linux 都可以开发，但不同平台可能还需要系统依赖；Windows 侧官方文档提供了 PowerShell bootstrap 脚本，Linux 侧官方文档提供了 `./script/bootstrap`。

## 为什么这样拆

- `main.rs`：只做应用初始化，保持干净。
- `app.rs`：主状态集中，包括当前文件、dirty 状态、模式切换、保存逻辑和 UI。
- `fs.rs`：文件读写与系统文件选择器隔离，之后可以替换为自己的工作区 / 数据库 / 自动保存。
- `theme.rs`：把 Floral Notepaper 的温暖纸张感抽象成 palette，后面要做设计规范或多主题时不需要改业务代码。

## 下一步建议

见 `docs/ROADMAP.md`。

## 注意

这是 starter scaffold。由于依赖来自 GitHub git 仓库，首次构建会拉取 `zed` 和 `gpui-component`，请确保网络能访问 GitHub。
