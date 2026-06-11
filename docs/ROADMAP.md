# Roadmap

## Phase 1：补齐便签体验

- 自动保存：编辑停止 500ms 后写入本地草稿
- 最近文件持久化：保存到系统 config dir
- 快捷键：`Ctrl/Cmd+N`、`Ctrl/Cmd+O`、`Ctrl/Cmd+S`、`Ctrl/Cmd+Shift+S`
- 关闭窗口前 dirty 检查
- 空状态和错误 toast

## Phase 2：对齐 Floral Notepaper 的核心模式

- 托盘常驻
- 全局快捷键唤起小窗
- 多便签窗口
- 磁贴 / 置顶窗口
- 透明度、窗口尺寸和位置记忆

## Phase 3：本地数据模型

- `notes/` 文件夹工作区
- SQLite / sled / redb 索引
- 标签、搜索、归档
- 每条 note 的 metadata：标题、创建时间、更新时间、窗口位置、是否置顶

## Phase 4：更完整的 Markdown

- Mermaid / 代码高亮主题
- 图片粘贴和本地附件管理
- GFM task list
- 导出 PDF / HTML

## Phase 5：工程化

- GitHub Actions 构建三平台包
- 单元测试：文件读写、recent list、dirty 状态
- e2e smoke test：启动窗口、打开/保存文件
- 安装包：Windows MSI/EXE、macOS DMG、Linux AppImage/deb/rpm
