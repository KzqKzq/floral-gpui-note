use crate::{fs, theme};
use gpui::*;
use gpui_component::{
    button::{Button, ButtonVariants},
    h_flex,
    input::{Input, InputEvent, InputState, TabSize},
    scroll::ScrollableElement as _,
    text::TextView,
    v_flex, Icon, IconName, Selectable, Sizable, TitleBar,
};
use std::time::Instant;

// ── Global actions ──
actions!(
    floral,
    [Save, NewNote, ToggleSidebar, ToggleTheme, OpenFloating]
);

// ── Toast notification ──
#[derive(Clone)]
struct Toast {
    message: SharedString,
    kind: ToastKind,
    created: Instant,
}

#[derive(Clone, Copy, PartialEq)]
enum ToastKind {
    Info,
    Success,
    Error,
}

const DEFAULT_NOTE: &str = r#"# 墨のち雨

伞を差して食み出した分だけ
> 撑起的伞 而没能遮住的部分

寄せかかったに また離れてく肩
> 渐渐靠近的肩 却又马上分开

押し合う手が真ん中で止まるように
> 紧紧相握的手 仿佛在我们间静止

押し返す強さを間違わないように
> 为了不搞错力气 而变得小心翼翼

降りかかった雨に冷やされないよう
> 为了这份温暖不被将要落下的雨冷却
"#;

const NOTE_CARD_WIDTH: f32 = 260.;
const DEFAULT_CATEGORY: &str = "";

#[derive(Clone, Copy)]
struct SeedNote {
    title: &'static str,
    category: &'static str,
    body: &'static str,
}

const SEED_NOTES: &[SeedNote] = &[
    SeedNote {
        title: "README",
        category: "",
        body: "# README\n\n# 花笺 一款 Windows 上轻量、优雅的本地便签工具。\n\n- 原生 GPUI 界面\n- 本地 Markdown 文件\n- 轻量编辑与实时预览\n",
    },
    SeedNote {
        title: "花记",
        category: "",
        body: "# 花记\n\n此时相望不相闻，愿逐月华流照君\n",
    },
    SeedNote {
        title: "关于花笺",
        category: "",
        body: "# 关于花笺\n\n- 设计主窗口 / 便签 / 磁贴三类窗口的统一生命周期。\n- 引入 Markdown 编辑和预览。\n- 保持启动快、界面轻、操作直接。\n",
    },
    SeedNote {
        title: "花笺开发记录",
        category: "开发",
        body: "# 花笺开发记录\n\n点击上面那个图钉按钮，会将笔记转为磁贴。\n\n> 现在先做本地编辑器，后续再接磁贴窗口。\n",
    },
    SeedNote {
        title: "大黑塔（线稿）",
        category: "素材",
        body: "# 大黑塔（线稿）\n\nthe_herta_\\(honkai_star_rail\\), purple eyes, brown hair\n\n- 线稿\n- 冷色阴影\n- 低饱和背景\n",
    },
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EditorMode {
    Edit,
    Preview,
    Split,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SidePanelMode {
    Settings,
    About,
    Floating,
}

pub struct FloralNotepad {
    editor: Entity<InputState>,
    search: Entity<InputState>,
    category_input: Entity<InputState>,
    store: fs::NoteStore,
    notes: Vec<fs::NoteMetadata>,
    categories: Vec<String>,
    current_note_id: Option<String>,
    current_category: String,
    dirty: bool,
    mode: EditorMode,
    status: SharedString,
    word_count: usize,
    sidebar_visible: bool,
    pinned: bool,
    line_numbers: bool,
    auto_save: bool,
    side_panel: Option<SidePanelMode>,
    notes_scroll: ScrollHandle,
    suppress_editor_change: bool,
    delete_pending: bool,
    delete_confirm_id: Option<String>,
    search_hits: usize,
    toast: Option<Toast>,
    dark_mode: bool,
    renaming_category: Option<String>,
    rename_input: Entity<InputState>,
    sidebar_width: f32,
    tab_size: usize,
}

impl FloralNotepad {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let initial_tab_size = 2usize;
        let editor = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor("markdown")
                .line_number(false)
                .searchable(true)
                .tab_size(TabSize {
                    tab_size: initial_tab_size,
                    hard_tabs: false,
                })
                .placeholder("写点 Markdown...")
                .default_value(DEFAULT_NOTE)
        });
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("搜索笔记..."));
        let category_input = cx.new(|cx| InputState::new(window, cx).placeholder("新分类名称"));
        let rename_input = cx.new(|cx| InputState::new(window, cx).placeholder("重命名分类"));

        let _editor_subscription =
            cx.subscribe_in(&editor, window, |this, input, event, _window, cx| {
                if matches!(event, InputEvent::Change) {
                    this.handle_editor_change(&input.read(cx).value(), cx);
                }
            });
        let _search_subscription =
            cx.subscribe_in(&search, window, |this, input, event, _window, cx| {
                if matches!(event, InputEvent::Change) {
                    let query = input.read(cx).value().trim().to_lowercase();
                    if query.is_empty() {
                        this.search_hits = 0;
                        this.status = "已清空搜索".into();
                    } else {
                        this.search_hits = this
                            .notes
                            .iter()
                            .filter(|note| {
                                let category = if note.category.is_empty() {
                                    "未分类"
                                } else {
                                    note.category.as_str()
                                };
                                FloralNotepad::matches_search(
                                    &query,
                                    &[&note.title, &note.preview, &note.file_name, category],
                                )
                            })
                            .count();
                        this.status =
                            format!("搜索 · {query} · {} 条结果", this.search_hits).into();
                    };
                    cx.notify();
                }
            });

        // ── Global keyboard shortcuts ──
        cx.bind_keys([
            KeyBinding::new("ctrl-s", Save, None),
            KeyBinding::new("ctrl-n", NewNote, None),
            KeyBinding::new("ctrl-b", ToggleSidebar, None),
            KeyBinding::new("ctrl-shift-t", ToggleTheme, None),
            KeyBinding::new("ctrl-shift-n", OpenFloating, None),
        ]);

        let mut this = Self {
            editor,
            search,
            category_input,
            store: fs::default_store(),
            notes: Vec::new(),
            categories: Vec::new(),
            current_note_id: None,
            current_category: DEFAULT_CATEGORY.to_string(),
            dirty: false,
            mode: EditorMode::Split,
            status: "正在载入笔记库".into(),
            word_count: fs::count_note_chars(DEFAULT_NOTE),
            sidebar_visible: true,
            pinned: false,
            line_numbers: false,
            auto_save: true,
            side_panel: None,
            notes_scroll: ScrollHandle::new(),
            suppress_editor_change: false,
            delete_pending: false,
            delete_confirm_id: None,
            search_hits: 0,
            toast: None,
            dark_mode: false,
            renaming_category: None,
            rename_input,
            sidebar_width: 286.,
            tab_size: 2,
        };

        this.bootstrap(window, cx);
        this
    }

    fn bootstrap(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Err(err) = self.seed_notes_if_empty() {
            self.status = format!("初始化示例笔记失败 · {err:#}").into();
        }
        self.refresh_library();

        if let Some(first_id) = self.notes.first().map(|note| note.id.clone()) {
            self.load_note_by_id(first_id, window, cx);
        } else {
            self.replace_editor_value("# 未命名笔记\n\n".to_string(), window, cx);
            self.status = "笔记库为空，已创建草稿".into();
        }
    }

    fn seed_notes_if_empty(&self) -> anyhow::Result<()> {
        if !self.store.list_notes()?.is_empty() {
            return Ok(());
        }

        for note in SEED_NOTES {
            self.store.create_note(fs::SaveNoteRequest {
                title: note.title.to_string(),
                content: note.body.to_string(),
                category: note.category.to_string(),
            })?;
        }
        Ok(())
    }

    fn refresh_library(&mut self) {
        match self.store.list_notes() {
            Ok(notes) => self.notes = notes,
            Err(err) => self.status = format!("读取笔记库失败 · {err:#}").into(),
        }
        match self.store.list_categories() {
            Ok(categories) => self.categories = categories,
            Err(err) => self.status = format!("读取分类失败 · {err:#}").into(),
        }
    }

    fn handle_editor_change(&mut self, value: &str, cx: &mut Context<Self>) {
        if self.suppress_editor_change {
            return;
        }

        self.delete_pending = false;
        self.word_count = fs::count_note_chars(value);
        self.dirty = true;
        if self.auto_save && self.current_note_id.is_some() {
            self.save_current_note(cx);
        } else {
            self.status = format!("已修改 · {} 字", self.word_count).into();
        }
        cx.notify();
    }

    fn replace_editor_value(&mut self, value: String, window: &mut Window, cx: &mut Context<Self>) {
        self.suppress_editor_change = true;
        self.editor.update(cx, |input, cx| {
            input.set_value(value.clone(), window, cx);
        });
        self.suppress_editor_change = false;
        self.word_count = fs::count_note_chars(&value);
    }

    fn current_markdown(&self, cx: &mut Context<Self>) -> String {
        self.editor.read(cx).value().to_string()
    }

    fn current_title(&self, cx: &mut Context<Self>) -> String {
        let markdown = self.current_markdown(cx);
        fs::normalize_note_title("", &markdown)
    }

    fn current_note(&self) -> Option<&fs::NoteMetadata> {
        let id = self.current_note_id.as_ref()?;
        self.notes.iter().find(|note| &note.id == id)
    }

    fn current_updated_at(&self) -> String {
        self.current_note()
            .map(|note| fs::format_full_time(note.updated_at))
            .unwrap_or_else(|| "草稿".to_string())
    }

    fn current_category_label(&self) -> String {
        if self.current_category.trim().is_empty() {
            "未分类".to_string()
        } else {
            self.current_category.clone()
        }
    }

    fn note_request(&self, cx: &mut Context<Self>) -> fs::SaveNoteRequest {
        let content = self.current_markdown(cx);
        fs::SaveNoteRequest {
            title: fs::normalize_note_title("", &content),
            content,
            category: self.current_category.clone(),
        }
    }

    fn save_current_note(&mut self, cx: &mut Context<Self>) {
        let request = self.note_request(cx);
        let result = if let Some(id) = self.current_note_id.clone() {
            self.store.update_note(&id, request)
        } else {
            self.store.create_note(request)
        };

        match result {
            Ok(note) => {
                self.current_note_id = Some(note.id.clone());
                self.current_category = note.category.clone();
                self.word_count = note.word_count;
                self.dirty = false;
                self.refresh_library();
                self.status = format!("已保存 · {}", note.title).into();
            }
            Err(err) => {
                self.status = format!("保存失败 · {err:#}").into();
            }
        }
        cx.notify();
    }

    fn save_before_switch(&mut self, cx: &mut Context<Self>) {
        if self.dirty && self.auto_save {
            self.save_current_note(cx);
        }
    }

    fn new_note(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.save_before_switch(cx);
        self.current_note_id = None;
        self.current_category = DEFAULT_CATEGORY.to_string();
        self.dirty = false;
        self.replace_editor_value("# 未命名笔记\n\n".to_string(), window, cx);
        self.status = "已新建草稿".into();
        cx.notify();
    }

    fn load_note_by_id(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        if self.current_note_id.as_deref() == Some(id.as_str()) {
            self.status = format!("已选中 · {}", self.current_title(cx)).into();
            cx.notify();
            return;
        }

        self.save_before_switch(cx);
        match self.store.read_note(&id) {
            Ok(note) => {
                self.replace_editor_value(note.content, window, cx);
                self.current_note_id = Some(note.id.clone());
                self.current_category = note.category.clone();
                self.word_count = note.word_count;
                self.dirty = false;
                self.status = format!("已打开 · {}", note.title).into();
            }
            Err(err) => {
                self.status = format!("打开失败 · {err:#}").into();
                self.refresh_library();
            }
        }
        cx.notify();
    }

    fn import_note(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(path) = fs::pick_markdown_file() else {
            self.status = "已取消导入".into();
            cx.notify();
            return;
        };

        self.save_before_switch(cx);
        match self
            .store
            .import_markdown_file(&path, &self.current_category)
        {
            Ok(note) => {
                self.current_note_id = Some(note.id.clone());
                self.current_category = note.category.clone();
                self.replace_editor_value(note.content, window, cx);
                self.dirty = false;
                self.refresh_library();
                self.status = format!("已导入 · {}", fs::display_name(&path)).into();
            }
            Err(err) => {
                self.status = format!("导入失败 · {err:#}").into();
            }
        }
        cx.notify();
    }

    fn export_note(&mut self, cx: &mut Context<Self>) {
        let title = self.current_title(cx);
        let Some(path) = fs::pick_export_path(&title) else {
            self.status = "已取消导出".into();
            cx.notify();
            return;
        };

        let note_id = self.current_note_id.clone();
        let result = if let Some(id) = note_id.as_deref() {
            if self.dirty {
                self.save_current_note(cx);
            }
            self.store.export_markdown_file(id, &path)
        } else {
            fs::write_markdown(&path, &self.current_markdown(cx))
        };

        self.status = match result {
            Ok(()) => format!("已导出 · {}", fs::display_name(&path)).into(),
            Err(err) => format!("导出失败 · {err:#}").into(),
        };
        cx.notify();
    }

    fn delete_note(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.current_note_id.is_none() {
            cx.notify();
            return;
        }
        // Show confirm dialog instead of immediate delete
        self.delete_confirm_id = self.current_note_id.clone();
        cx.notify();
    }

    fn confirm_delete_note(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let id = self.delete_confirm_id.take().unwrap_or_default();
        if id.is_empty() {
            cx.notify();
            return;
        }

        self.delete_pending = false;
        match self.store.delete_note(&id) {
            Ok(()) => {
                let was_current = self.current_note_id.as_deref() == Some(id.as_str());
                self.current_note_id = None;
                self.dirty = false;
                self.refresh_library();
                if was_current {
                    if let Some(next_id) = self.notes.first().map(|note| note.id.clone()) {
                        self.load_note_by_id(next_id, window, cx);
                    } else {
                        self.replace_editor_value("# 未命名笔记\n\n".to_string(), window, cx);
                        self.status = "已删除，当前为草稿".into();
                    }
                }
                self.toast = Some(Toast {
                    message: "笔记已删除".into(),
                    kind: ToastKind::Success,
                    created: Instant::now(),
                });
            }
            Err(err) => {
                self.toast = Some(Toast {
                    message: format!("删除失败 · {err:#}").into(),
                    kind: ToastKind::Error,
                    created: Instant::now(),
                });
            }
        }
        cx.notify();
    }

    fn cancel_delete(&mut self, cx: &mut Context<Self>) {
        self.delete_confirm_id = None;
        cx.notify();
    }

    fn delete_confirm_dialog(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let note_title = self
            .delete_confirm_id
            .as_ref()
            .and_then(|_| Some(self.current_title(cx)))
            .unwrap_or_else(|| "当前笔记".to_string());

        div()
            .absolute()
            .top_0()
            .left_0()
            .right_0()
            .bottom_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(hsla(0.0, 0.0, if self.dark_mode { 0.0 } else { 1.0 }, 0.58))
            .child(
                v_flex()
                    .w(px(400.))
                    .gap_5()
                    .p_8()
                    .rounded_xl()
                    .border_1()
                    .border_color(theme::line())
                    .bg(theme::paper())
                    .shadow_2xl()
                    // ═══ Title row ═══
                    .child(
                        h_flex()
                            .gap_3()
                            .items_center()
                            .child(
                                div()
                                    .w(px(10.))
                                    .h(px(10.))
                                    .flex_shrink_0()
                                    .rounded_full()
                                    .bg(theme::danger()),
                            )
                            .child(
                                div()
                                    .text_lg()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(theme::text())
                                    .child("确认删除"),
                            ),
                    )
                    // ═══ Body ═══
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme::text_muted())
                            .line_height(rems(1.6))
                            .child("此操作不可撤销。确定要删除以下笔记吗？"),
                    )
                    .child(
                        div()
                            .w_full()
                            .px_4()
                            .py_3()
                            .rounded_lg()
                            .border_1()
                            .border_color(theme::line())
                            .bg(theme::sidebar())
                            .child(
                                div()
                                    .w_full()
                                    .truncate()
                                    .text_sm()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(theme::text())
                                    .child(note_title),
                            ),
                    )
                    // ═══ Actions ─ right-aligned ═══
                    .child(
                        h_flex()
                            .w_full()
                            .gap_2()
                            .child(div().flex_1()) // spacer
                            .child(
                                Button::new("cancel-delete-dialog")
                                    .ghost()
                                    .rounded(px(8.))
                                    .label("取消")
                                    .on_click(cx.listener(|this, _event, _window, cx| {
                                        this.cancel_delete(cx);
                                    })),
                            )
                            .child(
                                Button::new("confirm-delete-dialog")
                                    .rounded(px(8.))
                                    .label("确认删除")
                                    .text_color(theme::paper())
                                    .bg(theme::danger())
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .on_click(cx.listener(|this, _event, window, cx| {
                                        this.confirm_delete_note(window, cx);
                                    })),
                            ),
                    ),
            )
    }

    fn open_floating_window(&mut self, cx: &mut Context<Self>) {
        self.side_panel = Some(SidePanelMode::Floating);
        cx.notify();
    }

    fn toggle_sidebar(&mut self, cx: &mut Context<Self>) {
        self.sidebar_visible = !self.sidebar_visible;
        self.status = if self.sidebar_visible {
            "已显示侧栏".into()
        } else {
            "已隐藏侧栏".into()
        };
        cx.notify();
    }

    fn toggle_pin(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.pinned = !self.pinned;
        if self.pinned {
            self.sidebar_visible = false;
            self.side_panel = None;
            window.resize(size(px(420.), px(560.)));
            self.status = "已切换为磁贴便签视图".into();
        } else {
            self.sidebar_visible = true;
            window.resize(size(px(1180.), px(760.)));
            self.status = "已恢复主窗口视图".into();
        }
        cx.notify();
    }

    fn show_note_info(&mut self, cx: &mut Context<Self>) {
        let category = self.current_category_label();
        let state = if self.dirty { "未保存" } else { "已保存" };
        self.status = format!(
            "{} · {} 字 · {} · {}",
            self.current_title(cx),
            self.word_count,
            category,
            state
        )
        .into();
        cx.notify();
    }

    fn toggle_line_numbers(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.line_numbers = !self.line_numbers;
        let line_numbers = self.line_numbers;
        self.editor.update(cx, |input, cx| {
            input.set_line_number(line_numbers, window, cx);
        });
        self.status = if line_numbers {
            "已显示行号".into()
        } else {
            "已隐藏行号".into()
        };
        cx.notify();
    }

    fn toggle_auto_save(&mut self, cx: &mut Context<Self>) {
        self.auto_save = !self.auto_save;
        if self.auto_save && self.dirty {
            self.save_current_note(cx);
        }
        self.status = if self.auto_save {
            "已开启自动保存".into()
        } else {
            "已关闭自动保存".into()
        };
        cx.notify();
    }

    fn toggle_side_panel(&mut self, mode: SidePanelMode, cx: &mut Context<Self>) {
        self.side_panel = if self.side_panel == Some(mode) {
            None
        } else {
            Some(mode)
        };
        cx.notify();
    }

    fn create_category_from_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = self.category_input.read(cx).value().trim().to_string();
        match self.store.create_category(&name) {
            Ok(()) => {
                self.current_category = name.clone();
                self.category_input.update(cx, |input, cx| {
                    input.set_value("", window, cx);
                });
                self.refresh_library();
                self.status = format!("已创建分类 · {name}").into();
            }
            Err(err) => {
                self.status = format!("创建分类失败 · {err:#}").into();
            }
        }
        cx.notify();
    }

    fn rename_category(&mut self, old_name: String, new_name: String, cx: &mut Context<Self>) {
        match self.store.rename_category(&old_name, &new_name) {
            Ok(()) => {
                if self.current_category == old_name {
                    self.current_category = new_name.clone();
                }
                self.refresh_library();
                self.renaming_category = None;
                self.toast = Some(Toast {
                    message: format!("已重命名为「{new_name}」").into(),
                    kind: ToastKind::Success,
                    created: Instant::now(),
                });
            }
            Err(err) => {
                self.toast = Some(Toast {
                    message: format!("重命名失败 · {err:#}").into(),
                    kind: ToastKind::Error,
                    created: Instant::now(),
                });
            }
        }
        cx.notify();
    }

    fn delete_category(&mut self, name: String, cx: &mut Context<Self>) {
        match self.store.delete_category(&name) {
            Ok(()) => {
                if self.current_category == name {
                    self.current_category = DEFAULT_CATEGORY.to_string();
                }
                self.refresh_library();
                self.toast = Some(Toast {
                    message: format!("已删除分类「{name}」").into(),
                    kind: ToastKind::Success,
                    created: Instant::now(),
                });
            }
            Err(err) => {
                self.toast = Some(Toast {
                    message: format!("删除分类失败 · {err:#}").into(),
                    kind: ToastKind::Error,
                    created: Instant::now(),
                });
            }
        }
        cx.notify();
    }

    fn move_current_to_category(&mut self, category: String, cx: &mut Context<Self>) {
        self.current_category = category.clone();
        if let Some(id) = self.current_note_id.clone() {
            if self.dirty {
                self.save_current_note(cx);
            }
            match self.store.move_note_to_category(&id, &category) {
                Ok(_) => {
                    self.refresh_library();
                    self.status =
                        format!("已移动到分类 · {}", self.current_category_label()).into();
                }
                Err(err) => self.status = format!("移动分类失败 · {err:#}").into(),
            }
        } else {
            self.status = format!("草稿分类 · {}", self.current_category_label()).into();
        }
        cx.notify();
    }

    fn set_mode(&mut self, mode: EditorMode, cx: &mut Context<Self>) {
        self.mode = mode;
        self.status = format!("已切换到{}", self.mode_label()).into();
        cx.notify();
    }

    fn insert_markdown(
        &mut self,
        snippet: &'static str,
        action: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.editor.update(cx, |input, cx| {
            input.insert(snippet, window, cx);
        });
        self.word_count = fs::count_note_chars(&self.current_markdown(cx));
        self.dirty = true;
        if self.auto_save && self.current_note_id.is_some() {
            self.save_current_note(cx);
        } else {
            self.status = format!("已插入 · {action}").into();
        }
        cx.notify();
    }

    fn mode_label(&self) -> &'static str {
        match self.mode {
            EditorMode::Edit => "编辑",
            EditorMode::Split => "分栏",
            EditorMode::Preview => "预览",
        }
    }

    fn search_query(&self, cx: &mut Context<Self>) -> String {
        self.search.read(cx).value().trim().to_lowercase()
    }

    fn matches_search(query: &str, fields: &[&str]) -> bool {
        query.is_empty()
            || fields
                .iter()
                .any(|field| field.to_lowercase().contains(query))
    }

    fn titlebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        TitleBar::new()
            .bg(theme::chrome())
            .border_color(theme::line())
            .child(
                h_flex()
                    .w_full()
                    .h_full()
                    .items_center()
                    .gap_3()
                    .pr_3()
                    .child(
                        div()
                            .text_color(theme::text())
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("花笺"),
                    )
                    .child(div().text_color(theme::text_faint()).text_sm().child("—"))
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme::text_muted())
                            .truncate()
                            .child(self.current_title(cx)),
                    )
                    .child(
                        h_flex()
                            .ml_auto()
                            .items_center()
                            .gap_2()
                            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                            .child(
                                Button::new("title-about")
                                    .ghost()
                                    .small()
                                    .compact()
                                    .icon(IconName::Info)
                                    .tooltip("关于")
                                    .on_click(cx.listener(|this, _event, _window, cx| {
                                        this.toggle_side_panel(SidePanelMode::About, cx)
                                    })),
                            )
                            .child(
                                Button::new("title-settings")
                                    .ghost()
                                    .small()
                                    .compact()
                                    .icon(IconName::Settings2)
                                    .tooltip("设置")
                                    .selected(self.side_panel == Some(SidePanelMode::Settings))
                                    .on_click(cx.listener(|this, _event, _window, cx| {
                                        this.toggle_side_panel(SidePanelMode::Settings, cx)
                                    })),
                            ),
                    ),
            )
    }

    fn sidebar_action(
        &self,
        id: &'static str,
        icon: IconName,
        label: &'static str,
        accent: bool,
        cx: &mut Context<Self>,
    ) -> Button {
        let button = Button::new(id)
            .ghost()
            .small()
            .rounded(px(8.))
            .icon(icon)
            .label(label);

        let button = if accent {
            button
                .text_color(theme::accent_dark())
                .font_weight(FontWeight::SEMIBOLD)
        } else {
            button.text_color(theme::text_muted())
        };

        match id {
            "new-note-action" => {
                button.on_click(cx.listener(|this, _event, window, cx| this.new_note(window, cx)))
            }
            "import-note-action" => button
                .on_click(cx.listener(|this, _event, window, cx| this.import_note(window, cx))),
            "export-note-action" => {
                button.on_click(cx.listener(|this, _event, _window, cx| this.export_note(cx)))
            }
            _ => button,
        }
    }

    fn note_card(
        &self,
        id: impl Into<ElementId>,
        selected: bool,
        title: impl Into<SharedString>,
        date: impl Into<SharedString>,
        excerpt: impl Into<SharedString>,
        meta: impl Into<SharedString>,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> impl IntoElement {
        let title = title.into();
        let date = date.into();
        let excerpt = excerpt.into();
        let meta = meta.into();

        h_flex()
            .id(id)
            .w(px(NOTE_CARD_WIDTH))
            .min_w(px(NOTE_CARD_WIDTH))
            .flex_shrink_0()
            .items_center()
            .gap_2()
            .rounded_lg()
            .overflow_hidden()
            .bg(if selected {
                theme::selected_note()
            } else {
                theme::transparent()
            })
            .cursor_pointer()
            .hover(|style| style.bg(theme::selected_note()))
            .on_click(on_click)
            .child(
                div()
                    .w(px(3.))
                    .h(px(38.))
                    .flex_shrink_0()
                    .rounded_full()
                    .bg(if selected {
                        theme::accent()
                    } else {
                        theme::transparent()
                    }),
            )
            .child(
                v_flex()
                    .w_full()
                    .flex_1()
                    .min_w_0()
                    .gap_2()
                    .p_3()
                    .rounded_lg()
                    .child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .flex_1()
                                    .truncate()
                                    .text_color(if selected {
                                        theme::accent_dark()
                                    } else {
                                        theme::text()
                                    })
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(title),
                            )
                            .child(
                                div()
                                    .flex_shrink_0()
                                    .text_xs()
                                    .text_color(theme::text_faint())
                                    .child(date),
                            ),
                    )
                    .child(
                        div()
                            .w_full()
                            .overflow_hidden()
                            .text_xs()
                            .line_clamp(2)
                            .line_height(rems(1.35))
                            .text_color(theme::text_faint())
                            .child(excerpt),
                    )
                    .child(
                        div()
                            .w_full()
                            .truncate()
                            .text_xs()
                            .text_color(theme::text_faint())
                            .child(meta),
                    ),
            )
    }

    fn sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let query = self.search_query(cx);
        let mut filtered_notes = self
            .notes
            .iter()
            .filter(|note| {
                let category = if note.category.is_empty() {
                    "未分类"
                } else {
                    note.category.as_str()
                };
                Self::matches_search(
                    &query,
                    &[&note.title, &note.preview, &note.file_name, category],
                )
            })
            .cloned()
            .collect::<Vec<_>>();
        filtered_notes.sort_by(|a, b| {
            let a_empty = a.category.is_empty();
            let b_empty = b.category.is_empty();
            match (a_empty, b_empty) {
                (true, false) => std::cmp::Ordering::Greater,
                (false, true) => std::cmp::Ordering::Less,
                _ => {
                    let category_cmp = a.category.cmp(&b.category);
                    if category_cmp == std::cmp::Ordering::Equal {
                        b.updated_at.cmp(&a.updated_at)
                    } else {
                        category_cmp
                    }
                }
            }
        });

        let mut notes_content = v_flex().id("notes-list").w_full().gap_3().pr_3();
        let mut last_category: Option<String> = None;
        for note in filtered_notes.iter() {
            let category_label = if note.category.is_empty() {
                "未分类".to_string()
            } else {
                note.category.clone()
            };
            if last_category.as_deref() != Some(category_label.as_str()) {
                last_category = Some(category_label.clone());
                notes_content = notes_content.child(
                    div()
                        .pt_1()
                        .text_xs()
                        .text_color(theme::text_faint())
                        .child(category_label.clone()),
                );
            }

            let selected = self.current_note_id.as_deref() == Some(note.id.as_str());
            let note_id = note.id.clone();
            notes_content = notes_content.child(self.note_card(
                format!("note-{}", note.id),
                selected,
                note.title.clone(),
                fs::format_short_date(note.updated_at),
                note.preview.clone(),
                format!(
                    "{} · {} 字",
                    fs::format_time(note.updated_at),
                    note.word_count
                ),
                cx.listener(move |this, _event, window, cx| {
                    this.load_note_by_id(note_id.clone(), window, cx)
                }),
            ));
        }

        v_flex()
            .w(px(self.sidebar_width))
            .h_full()
            .flex_shrink_0()
            .gap_4()
            .pl_4()
            .pr_1()
            .py_4()
            .border_r_1()
            .border_color(theme::line())
            .bg(theme::sidebar())
            .child(
                div()
                    .h(px(38.))
                    .flex_shrink_0()
                    .rounded_lg()
                    .bg(theme::sidebar_control())
                    .px_2()
                    .child(
                        Input::new(&self.search)
                            .small()
                            .appearance(false)
                            .prefix(Icon::new(IconName::Search).small())
                            .w_full(),
                    ),
            )
            .child(
                v_flex()
                    .flex_shrink_0()
                    .gap_2()
                    .child(self.sidebar_action(
                        "new-note-action",
                        IconName::Plus,
                        "新建笔记",
                        true,
                        cx,
                    ))
                    .child(self.sidebar_action(
                        "import-note-action",
                        IconName::FolderOpen,
                        "导入 Markdown",
                        false,
                        cx,
                    ))
                    .child(self.sidebar_action(
                        "export-note-action",
                        IconName::FolderOpen,
                        "导出当前笔记",
                        false,
                        cx,
                    )),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(theme::text_faint())
                    .child(SharedString::from(if self.search_hits > 0 {
                        format!("{} / {} 篇笔记", self.search_hits, self.notes.len())
                    } else {
                        format!("{} 篇笔记", filtered_notes.len())
                    })),
            )
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .child(
                        div()
                            .id("notes-scroll-area")
                            .size_full()
                            .overflow_y_scroll()
                            .track_scroll(&self.notes_scroll)
                            .child(notes_content),
                    )
                    .vertical_scrollbar(&self.notes_scroll),
            )
    }

    fn mode_button(
        &self,
        id: &'static str,
        label: &'static str,
        mode: EditorMode,
        cx: &mut Context<Self>,
    ) -> Button {
        Button::new(id)
            .ghost()
            .small()
            .compact()
            .rounded(px(8.))
            .selected(self.mode == mode)
            .label(label)
            .on_click(cx.listener(move |this, _event, _window, cx| this.set_mode(mode, cx)))
    }

    fn workspace_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .h(px(58.))
            .flex_shrink_0()
            .items_center()
            .gap_3()
            .px_6()
            .border_b_1()
            .border_color(theme::line())
            .bg(theme::paper())
            .child(
                Button::new("sidebar-visual")
                    .ghost()
                    .small()
                    .compact()
                    .icon(IconName::PanelLeft)
                    .tooltip("显示/隐藏侧栏")
                    .selected(!self.sidebar_visible)
                    .on_click(cx.listener(|this, _event, _window, cx| this.toggle_sidebar(cx))),
            )
            .child(div().w(px(1.)).h(px(18.)).bg(theme::line()))
            .child(
                Button::new("pin-note")
                    .ghost()
                    .small()
                    .compact()
                    .icon(IconName::Asterisk)
                    .tooltip("磁贴便签视图")
                    .selected(self.pinned)
                    .text_color(if self.pinned {
                        theme::accent_dark()
                    } else {
                        theme::text_faint()
                    })
                    .on_click(cx.listener(|this, _event, window, cx| this.toggle_pin(window, cx))),
            )
            .child(
                Button::new("save-note")
                    .ghost()
                    .small()
                    .rounded(px(8.))
                    .label("保存")
                    .on_click(cx.listener(|this, _event, _window, cx| this.save_current_note(cx))),
            )
            .child(
                Button::new("export-note")
                    .ghost()
                    .small()
                    .rounded(px(8.))
                    .label("导出")
                    .on_click(cx.listener(|this, _event, _window, cx| this.export_note(cx))),
            )
            .child(
                Button::new("delete-note")
                    .ghost()
                    .small()
                    .compact()
                    .icon(IconName::Delete)
                    .tooltip("删除当前笔记")
                    .on_click(cx.listener(|this, _event, window, cx| this.delete_note(window, cx))),
            )
            .child(
                Button::new("floating-notepad")
                    .ghost()
                    .small()
                    .compact()
                    .icon(IconName::PanelRightOpen)
                    .tooltip("浮动便签 (Ctrl+Shift+N)")
                    .on_click(cx.listener(|this, _event, _window, cx| {
                        this.open_floating_window(cx);
                    })),
            )
            .child(
                Button::new("note-info")
                    .ghost()
                    .small()
                    .compact()
                    .icon(IconName::Info)
                    .tooltip("当前笔记信息")
                    .on_click(cx.listener(|this, _event, _window, cx| this.show_note_info(cx))),
            )
            .child(
                h_flex()
                    .ml_auto()
                    .items_center()
                    .gap_1()
                    .p_1()
                    .rounded_lg()
                    .bg(theme::control())
                    .child(self.mode_button("mode-edit", "编辑", EditorMode::Edit, cx))
                    .child(self.mode_button("mode-split", "分栏", EditorMode::Split, cx))
                    .child(self.mode_button("mode-preview", "预览", EditorMode::Preview, cx)),
            )
    }

    fn note_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .flex_shrink_0()
            .items_end()
            .px_6()
            .pt_6()
            .pb_4()
            .border_b_1()
            .border_color(theme::line())
            .bg(theme::paper())
            .child(
                v_flex()
                    .gap_3()
                    .child(
                        div()
                            .font_family("SimSun")
                            .text_2xl()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme::text())
                            .child(self.current_title(cx)),
                    )
                    .child(
                        h_flex()
                            .gap_4()
                            .text_xs()
                            .text_color(theme::text_faint())
                            .child(self.current_updated_at())
                            .child("·")
                            .child(format!("{} 字", self.word_count))
                            .child("·")
                            .child(self.current_category_label())
                            .child("·")
                            .child(
                                div()
                                    .text_color(if self.dirty {
                                        theme::danger()
                                    } else {
                                        theme::text_faint()
                                    })
                                    .child(if self.dirty { "未保存" } else { "已保存" }),
                            ),
                    ),
            )
            .child(
                div()
                    .ml_auto()
                    .text_xs()
                    .text_color(theme::text_faint())
                    .child(self.mode_label()),
            )
    }

    fn format_button(
        &self,
        id: &'static str,
        label: &'static str,
        snippet: &'static str,
        action: &'static str,
        cx: &mut Context<Self>,
    ) -> Button {
        Button::new(id)
            .ghost()
            .small()
            .compact()
            .rounded(px(6.))
            .label(label)
            .on_click(cx.listener(move |this, _event, window, cx| {
                this.insert_markdown(snippet, action, window, cx)
            }))
    }

    fn format_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .h(px(34.))
            .items_center()
            .gap_2()
            .px_5()
            .text_xs()
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(theme::text_faint())
            .child(self.format_button("fmt-bold", "B", "**加粗文本**", "加粗", cx))
            .child(self.format_button("fmt-italic", "/", "*斜体文本*", "斜体", cx))
            .child(self.format_button("fmt-heading", "H", "\n## 小标题\n", "标题", cx))
            .child(self.format_button("fmt-rule", "—", "\n---\n", "分割线", cx))
            .child(self.format_button("fmt-list", "•", "\n- 列表项", "无序列表", cx))
            .child(self.format_button("fmt-ordered", "1.", "\n1. 列表项", "有序列表", cx))
            .child(self.format_button("fmt-code", "<>", "`代码`", "行内代码", cx))
            .child(self.format_button("fmt-quote", "“", "\n> 引用\n", "引用", cx))
            .child(self.format_button("fmt-math", "$", "$E=mc^2$", "行内公式", cx))
            .child(self.format_button(
                "fmt-block-math",
                "$$",
                "\n$$\nx^2 + y^2 = r^2\n$$\n",
                "公式块",
                cx,
            ))
    }

    fn editor_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .flex_1()
            .h_full()
            .bg(theme::paper())
            .child(self.format_toolbar(cx))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .px_5()
                    .pb_5()
                    .bg(theme::paper())
                    .child(
                        div()
                            .size_full()
                            .bg(theme::paper())
                            .child(Input::new(&self.editor).h_full().w_full()),
                    ),
            )
    }

    fn preview_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let markdown = self.current_markdown(cx);

        v_flex()
            .flex_1()
            .h_full()
            .gap_4()
            .px_6()
            .py_5()
            .bg(theme::paper())
            .child(
                div()
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme::text_faint())
                    .child("PREVIEW"),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scrollbar()
                    .text_color(theme::text())
                    .child(
                        TextView::markdown("markdown-preview", markdown)
                            .selectable(true)
                            .size_full(),
                    ),
            )
    }

    fn editor_content(&self, cx: &mut Context<Self>) -> impl IntoElement {
        match self.mode {
            EditorMode::Edit => h_flex()
                .flex_1()
                .overflow_hidden()
                .child(self.editor_panel(cx)),
            EditorMode::Preview => h_flex()
                .flex_1()
                .overflow_hidden()
                .child(self.preview_panel(cx)),
            EditorMode::Split => h_flex()
                .flex_1()
                .overflow_hidden()
                .child(self.editor_panel(cx))
                .child(div().w(px(3.)).my_5().rounded_full().bg(theme::divider()))
                .child(self.preview_panel(cx)),
        }
    }

    fn status_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let line_count = self.current_markdown(cx).lines().count().max(1);
        h_flex()
            .h(px(28.))
            .flex_shrink_0()
            .items_center()
            .px_5()
            .border_t_1()
            .border_color(theme::line())
            .bg(theme::paper())
            .text_xs()
            .text_color(theme::text_faint())
            .child(format!("{} 行", line_count))
            .child(div().mx_4().w(px(1.)).h(px(14.)).bg(theme::line()))
            .child("Markdown")
            .child(div().mx_4().w(px(1.)).h(px(14.)).bg(theme::line()))
            .child(div().flex_1().truncate().child(self.status.clone()))
            .child(div().ml_auto().child(self.current_category_label()))
            .child(div().mx_4().w(px(1.)).h(px(14.)).bg(theme::line()))
            .child(if self.auto_save { "Auto" } else { "Manual" })
    }

    fn settings_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut categories = v_flex().gap_2();
        categories = categories.child(self.category_button("category-default", "未分类", "", cx));
        for category in &self.categories {
            let cat = category.clone();
            let cat2 = cat.clone();
            let cat3 = cat.clone();
            categories = categories.child(
                h_flex()
                    .gap_1()
                    .items_center()
                    .child(self.category_button(
                        format!("cat-btn-{cat}"),
                        cat.clone(),
                        cat.clone(),
                        cx,
                    ))
                    .child(
                        Button::new(format!("rename-cat-{cat}"))
                            .ghost()
                            .small()
                            .compact()
                            .icon(IconName::File)
                            .tooltip("重命名分类")
                            .on_click(cx.listener(move |this, _event, _window, cx| {
                                this.renaming_category = Some(cat2.clone());
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new(format!("delete-cat-{cat}"))
                            .ghost()
                            .small()
                            .compact()
                            .icon(IconName::Delete)
                            .tooltip("删除分类")
                            .text_color(theme::danger())
                            .on_click(cx.listener(move |this, _event, _window, cx| {
                                this.delete_category(cat3.clone(), cx);
                            })),
                    ),
            );
        }

        // Rename inline input
        let rename_section = if let Some(ref old_name) = self.renaming_category {
            Some(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .h(px(34.))
                            .flex_1()
                            .rounded_lg()
                            .bg(theme::sidebar_control())
                            .px_2()
                            .child(
                                Input::new(&self.rename_input)
                                    .small()
                                    .appearance(false)
                                    .w_full(),
                            ),
                    )
                    .child(
                        Button::new("confirm-rename")
                            .ghost()
                            .small()
                            .rounded(px(8.))
                            .icon(IconName::Check)
                            .on_click({
                                let old = old_name.clone();
                                cx.listener(move |this, _event, _window, cx| {
                                    let new_name =
                                        this.rename_input.read(cx).value().trim().to_string();
                                    this.rename_category(old.clone(), new_name, cx);
                                })
                            }),
                    )
                    .child(
                        Button::new("cancel-rename")
                            .ghost()
                            .small()
                            .compact()
                            .icon(IconName::Close)
                            .on_click(cx.listener(|this, _event, _window, cx| {
                                this.renaming_category = None;
                                cx.notify();
                            })),
                    ),
            )
        } else {
            None
        };

        let mut panel = v_flex()
            .w(px(300.))
            .h_full()
            .flex_shrink_0()
            .gap_4()
            .p_5()
            .border_l_1()
            .border_color(theme::line())
            .bg(theme::sidebar())
            .child(
                h_flex()
                    .items_center()
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme::text())
                            .child("设置"),
                    )
                    .child(
                        Button::new("close-settings")
                            .ghost()
                            .small()
                            .compact()
                            .ml_auto()
                            .icon(IconName::Close)
                            .on_click(cx.listener(|this, _event, _window, cx| {
                                this.side_panel = None;
                                cx.notify();
                            })),
                    ),
            )
            .child(self.setting_toggle(
                "setting-autosave",
                "自动保存",
                self.auto_save,
                cx.listener(|this, _event, _window, cx| this.toggle_auto_save(cx)),
            ))
            .child(self.setting_toggle(
                "setting-darkmode",
                "暗色模式",
                self.dark_mode,
                cx.listener(|this, _event, _window, cx| {
                    this.dark_mode = !this.dark_mode;
                    cx.notify();
                }),
            ))
            .child(self.setting_toggle(
                "setting-line-number",
                "显示行号",
                self.line_numbers,
                cx.listener(|this, _event, window, cx| this.toggle_line_numbers(window, cx)),
            ))
            // ── Sidebar width ──
            .child(self.setting_slider(
                "sidebar-width",
                "侧栏宽度",
                self.sidebar_width,
                |this, cx| {
                    this.sidebar_width = (this.sidebar_width - 10.).max(180.);
                    cx.notify();
                },
                |this, cx| {
                    this.sidebar_width = (this.sidebar_width + 10.).min(500.);
                    cx.notify();
                },
                cx,
            ))
            // ── Tab size ──
            .child(self.setting_slider(
                "tab-size",
                "Tab 缩进",
                self.tab_size as f32,
                |this, cx| {
                    this.tab_size = (this.tab_size.saturating_sub(1)).max(1);
                    cx.notify();
                },
                |this, cx| {
                    this.tab_size = (this.tab_size + 1).min(8);
                    cx.notify();
                },
                cx,
            ))
            // ── Default view mode ──
            .child(
                v_flex()
                    .gap_2()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme::text_faint())
                            .child("默认视图"),
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            .child(self.mode_button(
                                "setting-mode-edit",
                                "编辑",
                                EditorMode::Edit,
                                cx,
                            ))
                            .child(self.mode_button(
                                "setting-mode-split",
                                "分栏",
                                EditorMode::Split,
                                cx,
                            ))
                            .child(self.mode_button(
                                "setting-mode-preview",
                                "预览",
                                EditorMode::Preview,
                                cx,
                            )),
                    ),
            )
            .child(
                v_flex()
                    .gap_2()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme::text_faint())
                            .child("当前分类"),
                    )
                    .child(categories),
            );

        // Insert rename section if active
        if let Some(rename_el) = rename_section {
            panel = panel.child(rename_el);
        }

        panel = panel
            .child(
                v_flex()
                    .gap_2()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme::text_faint())
                            .child("新建分类"),
                    )
                    .child(
                        div()
                            .h(px(34.))
                            .rounded_lg()
                            .bg(theme::sidebar_control())
                            .px_2()
                            .child(
                                Input::new(&self.category_input)
                                    .small()
                                    .appearance(false)
                                    .w_full(),
                            ),
                    )
                    .child(
                        Button::new("create-category")
                            .ghost()
                            .small()
                            .rounded(px(8.))
                            .icon(IconName::Plus)
                            .label("添加分类")
                            .on_click(cx.listener(|this, _event, window, cx| {
                                this.create_category_from_input(window, cx)
                            })),
                    ),
            )
            .child(
                v_flex()
                    .gap_1()
                    .text_xs()
                    .text_color(theme::text_faint())
                    .child("数据目录")
                    .child(
                        div()
                            .line_clamp(3)
                            .child(self.store.base_dir().display().to_string()),
                    )
                    .child("笔记目录")
                    .child(
                        div()
                            .line_clamp(3)
                            .child(self.store.notes_dir().display().to_string()),
                    ),
            );

        panel
    }

    fn category_button(
        &self,
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        category: impl Into<String>,
        cx: &mut Context<Self>,
    ) -> Button {
        let category = category.into();
        let selected = self.current_category == category;
        Button::new(id)
            .ghost()
            .small()
            .rounded(px(8.))
            .selected(selected)
            .label(label)
            .on_click(cx.listener(move |this, _event, _window, cx| {
                this.move_current_to_category(category.clone(), cx)
            }))
    }

    fn setting_toggle(
        &self,
        id: &'static str,
        label: &'static str,
        selected: bool,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> impl IntoElement {
        h_flex()
            .items_center()
            .gap_3()
            .child(
                Button::new(id)
                    .ghost()
                    .small()
                    .rounded(px(8.))
                    .selected(selected)
                    .label(if selected { "开" } else { "关" })
                    .on_click(on_click),
            )
            .child(div().text_sm().text_color(theme::text_muted()).child(label))
    }

    fn setting_slider(
        &self,
        id: &'static str,
        label: &'static str,
        value: f32,
        on_dec: impl Fn(&mut Self, &mut Context<Self>) + 'static,
        on_inc: impl Fn(&mut Self, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        h_flex()
            .items_center()
            .gap_3()
            .child(
                h_flex()
                    .gap_1()
                    .child(
                        Button::new(format!("settings-{id}-dec"))
                            .ghost()
                            .small()
                            .compact()
                            .icon(IconName::Minus)
                            .on_click(cx.listener(move |this, _event, _window, cx| {
                                on_dec(this, cx);
                            })),
                    )
                    .child(
                        div()
                            .w(px(40.))
                            .text_sm()
                            .text_center()
                            .text_color(theme::text())
                            .child(format!("{value:.0}")),
                    )
                    .child(
                        Button::new(format!("settings-{id}-inc"))
                            .ghost()
                            .small()
                            .compact()
                            .icon(IconName::Plus)
                            .on_click(cx.listener(move |this, _event, _window, cx| {
                                on_inc(this, cx);
                            })),
                    ),
            )
            .child(div().text_sm().text_color(theme::text_muted()).child(label))
    }

    fn floating_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let title = self.current_title(cx);

        div()
            .flex_1()
            .flex()
            .items_center()
            .justify_center()
            .bg(if self.dark_mode {
                hsla(40.0 / 360.0, 0.08, 0.12, 1.0)
            } else {
                theme::paper()
            })
            .child(
                // Centered card
                v_flex()
                    .w(px(540.))
                    .h(px(500.))
                    .rounded_xl()
                    .border_1()
                    .border_color(theme::line())
                    .bg(theme::paper())
                    .shadow_2xl()
                    .overflow_hidden()
                    .child(
                        // Title bar
                        h_flex()
                            .h(px(42.))
                            .flex_shrink_0()
                            .items_center()
                            .gap_3()
                            .px_4()
                            .border_b_1()
                            .border_color(theme::line())
                            .bg(theme::chrome())
                            .child(
                                div()
                                    .flex_1()
                                    .truncate()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(theme::text())
                                    .child(title),
                            )
                            .child(
                                Button::new("float-save-modal")
                                    .ghost()
                                    .small()
                                    .compact()
                                    .label("保存")
                                    .on_click(cx.listener(|this, _event, _window, cx| {
                                        this.save_current_note(cx);
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Button::new("float-close-modal")
                                    .ghost()
                                    .small()
                                    .compact()
                                    .icon(IconName::Close)
                                    .on_click(cx.listener(|this, _event, _window, cx| {
                                        this.side_panel = None;
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_h_0()
                            .p_4()
                            .overflow_hidden()
                            .child(Input::new(&self.editor).h_full().w_full()),
                    )
                    .child(
                        h_flex()
                            .h(px(28.))
                            .flex_shrink_0()
                            .items_center()
                            .px_4()
                            .border_t_1()
                            .border_color(theme::line())
                            .bg(theme::paper())
                            .text_xs()
                            .text_color(theme::text_faint())
                            .child(format!("{} 字", self.word_count))
                            .child(div().ml_auto().child(if self.dirty {
                                "未保存"
                            } else {
                                "已保存"
                            })),
                    ),
            )
    }

    fn about_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w(px(300.))
            .h_full()
            .flex_shrink_0()
            .gap_4()
            .p_5()
            .border_l_1()
            .border_color(theme::line())
            .bg(theme::sidebar())
            .child(
                h_flex()
                    .items_center()
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme::text())
                            .child("关于花笺"),
                    )
                    .child(
                        Button::new("close-about")
                            .ghost()
                            .small()
                            .compact()
                            .ml_auto()
                            .icon(IconName::Close)
                            .on_click(cx.listener(|this, _event, _window, cx| {
                                this.side_panel = None;
                                cx.notify();
                            })),
                    ),
            )
            .child(
                div()
                    .font_family("SimSun")
                    .text_2xl()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme::text())
                    .child("花笺"),
            )
            .child(
                v_flex()
                    .gap_2()
                    .text_sm()
                    .line_height(rems(1.45))
                    .text_color(theme::text_muted())
                    .child("轻量、优雅、现代化的本地 Markdown 便签工具。")
                    .child("此 GPUI 版本已经移植主窗口笔记库、导入导出、实时预览、分类、自动保存和磁贴便签视图。"),
            )
            .child(
                v_flex()
                    .gap_1()
                    .text_xs()
                    .text_color(theme::text_faint())
                    .child(format!("笔记数量 · {}", self.notes.len()))
                    .child(format!("分类数量 · {}", self.categories.len()))
                    .child(format!("当前字数 · {}", self.word_count)),
            )
    }

    fn workspace(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .flex_1()
            .h_full()
            .overflow_hidden()
            .bg(theme::paper())
            .child(self.workspace_toolbar(cx))
            .child(self.note_header(cx))
            .child(self.editor_content(cx))
            .child(self.status_bar(cx))
    }

    fn tile_workspace(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .flex_1()
            .h_full()
            .overflow_hidden()
            .bg(theme::selected_note())
            .child(
                h_flex()
                    .h(px(42.))
                    .flex_shrink_0()
                    .items_center()
                    .gap_2()
                    .px_4()
                    .child(
                        div()
                            .flex_1()
                            .truncate()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme::accent_dark())
                            .child(self.current_title(cx)),
                    )
                    .child(
                        Button::new("tile-save")
                            .ghost()
                            .small()
                            .compact()
                            .label("保存")
                            .tooltip("保存")
                            .on_click(
                                cx.listener(|this, _event, _window, cx| this.save_current_note(cx)),
                            ),
                    )
                    .child(
                        Button::new("tile-unpin")
                            .ghost()
                            .small()
                            .compact()
                            .icon(IconName::Asterisk)
                            .tooltip("返回主窗口")
                            .on_click(
                                cx.listener(|this, _event, window, cx| this.toggle_pin(window, cx)),
                            ),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .p_4()
                    .child(Input::new(&self.editor).h_full().w_full()),
            )
            .child(
                h_flex()
                    .h(px(28.))
                    .px_4()
                    .text_xs()
                    .text_color(theme::text_faint())
                    .child(format!("{} 字", self.word_count))
                    .child(div().ml_auto().child(if self.dirty {
                        "未保存"
                    } else {
                        "已保存"
                    })),
            )
    }
}

impl Render for FloralNotepad {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // ── Poll global hotkey flag ──
        #[cfg(target_os = "windows")]
        if crate::platform::HOTKEY_FLAG.swap(false, std::sync::atomic::Ordering::SeqCst) {
            if self.side_panel != Some(SidePanelMode::Floating) {
                self.side_panel = Some(SidePanelMode::Floating);
            } else {
                self.side_panel = None;
            }
        }

        let bg_color = if self.dark_mode {
            hsla(40.0 / 360.0, 0.08, 0.12, 1.0)
        } else {
            theme::paper()
        };

        // ── Toast ──
        let toast_element = self.toast.as_ref().and_then(|toast| {
            if toast.created.elapsed().as_secs() < 3 {
                let toast_color = match toast.kind {
                    ToastKind::Success => theme::accent(),
                    ToastKind::Error => theme::danger(),
                    ToastKind::Info => theme::text_muted(),
                };
                Some(
                    div()
                        .absolute()
                        .top(px(58.))
                        .right(px(24.))
                        .px_4()
                        .py_2()
                        .rounded_lg()
                        .bg(if self.dark_mode {
                            hsla(0.0, 0.0, 0.18, 0.95)
                        } else {
                            hsla(0.0, 0.0, 0.02, 0.90)
                        })
                        .text_sm()
                        .text_color(toast_color)
                        .shadow_lg()
                        .child(toast.message.clone()),
                )
            } else {
                None
            }
        });

        if self.pinned {
            return v_flex()
                .size_full()
                .overflow_hidden()
                .bg(theme::selected_note())
                .font_family("Microsoft YaHei UI")
                .child(self.titlebar(cx))
                .child(self.tile_workspace(cx));
        }

        // ── Delete confirm: replace entire view ──
        // ── Floating modal: replace entire view ──
        if self.side_panel == Some(SidePanelMode::Floating) {
            return v_flex()
                .size_full()
                .bg(bg_color)
                .font_family("Microsoft YaHei UI")
                .child(self.titlebar(cx))
                .child(self.floating_panel(cx));
        }

        let mut body = h_flex().flex_1().overflow_hidden();
        if self.sidebar_visible {
            body = body.child(self.sidebar(cx));
        }
        body = body.child(self.workspace(cx));
        if let Some(panel) = self.side_panel {
            match panel {
                SidePanelMode::Settings => body = body.child(self.settings_panel(cx)),
                SidePanelMode::About => body = body.child(self.about_panel(cx)),
                _ => {}
            }
        }

        let main = v_flex()
            .size_full()
            .overflow_hidden()
            .bg(bg_color)
            .font_family("Microsoft YaHei UI")
            .on_action(cx.listener(|this, _: &Save, _window, cx| {
                this.save_current_note(cx);
                this.toast = Some(Toast {
                    message: "已保存".into(),
                    kind: ToastKind::Success,
                    created: Instant::now(),
                });
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &NewNote, window, cx| {
                this.new_note(window, cx);
                this.toast = Some(Toast {
                    message: "已新建便签".into(),
                    kind: ToastKind::Info,
                    created: Instant::now(),
                });
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &ToggleSidebar, _window, cx| {
                this.toggle_sidebar(cx);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &ToggleTheme, _window, cx| {
                this.dark_mode = !this.dark_mode;
                this.toast = Some(Toast {
                    message: if this.dark_mode {
                        "已切换为暗色主题".into()
                    } else {
                        "已切换为亮色主题".into()
                    },
                    kind: ToastKind::Info,
                    created: Instant::now(),
                });
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &OpenFloating, _window, cx| {
                this.open_floating_window(cx);
            }))
            .child(self.titlebar(cx))
            .child(body);

        let mut root = div().relative().size_full().child(main);
        if self.delete_confirm_id.is_some() {
            root = root.child(self.delete_confirm_dialog(cx));
        }
        if let Some(toast_el) = toast_element {
            root = root.child(toast_el);
        }
        root
    }
}
