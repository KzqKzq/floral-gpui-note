use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NoteMetadata {
    pub id: String,
    pub title: String,
    pub file_name: String,
    #[serde(default)]
    pub category: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub word_count: usize,
    pub preview: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub id: String,
    pub title: String,
    pub file_name: String,
    #[serde(default)]
    pub category: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub word_count: usize,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct SaveNoteRequest {
    pub title: String,
    pub content: String,
    pub category: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct MetadataFile {
    notes: Vec<NoteMetadata>,
    #[serde(default)]
    categories: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct NoteStore {
    base_dir: PathBuf,
    notes_dir: PathBuf,
}

pub fn pick_markdown_file() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter("Markdown", &["md", "markdown", "txt"])
        .pick_file()
}

pub fn pick_export_path(title: &str) -> Option<PathBuf> {
    let file_name = format!(
        "{}.md",
        safe_file_stem(title).unwrap_or_else(|| "untitled".into())
    );
    normalize_markdown_extension(
        rfd::FileDialog::new()
            .add_filter("Markdown", &["md", "markdown"])
            .set_file_name(file_name)
            .save_file()?,
    )
}

pub fn pick_folder() -> Option<PathBuf> {
    rfd::FileDialog::new().pick_folder()
}

fn normalize_markdown_extension(mut path: PathBuf) -> Option<PathBuf> {
    if path.extension().is_none() {
        path.set_extension("md");
    }
    Some(path)
}

pub fn read_markdown(path: &Path) -> Result<String> {
    fs::read_to_string(path).with_context(|| format!("读取文件失败: {}", path.display()))
}

pub fn write_markdown(path: &Path, markdown: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("创建目录失败: {}", parent.display()))?;
    }
    fs::write(path, markdown).with_context(|| format!("写入文件失败: {}", path.display()))
}

pub fn display_name(path: &Path) -> String {
    path.file_name()
        .and_then(|v| v.to_str())
        .unwrap_or("untitled.md")
        .to_string()
}

pub fn default_store() -> NoteStore {
    NoteStore::new(default_base_dir())
}

fn default_base_dir() -> PathBuf {
    if let Ok(path) = std::env::var("FLORAL_NOTEPAPER_DATA_DIR") {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }

    dirs::document_dir()
        .or_else(dirs::data_dir)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .join("花笺")
}

impl NoteStore {
    pub fn new(base_dir: PathBuf) -> Self {
        Self {
            notes_dir: base_dir.join("notes"),
            base_dir,
        }
    }

    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    pub fn notes_dir(&self) -> &Path {
        &self.notes_dir
    }

    fn metadata_path(&self) -> PathBuf {
        self.base_dir.join("metadata.json")
    }

    fn ensure_storage(&self) -> Result<()> {
        fs::create_dir_all(&self.notes_dir)
            .with_context(|| format!("创建笔记目录失败: {}", self.notes_dir.display()))
    }

    fn load_metadata(&self) -> Result<MetadataFile> {
        self.ensure_storage()?;
        let path = self.metadata_path();
        if !path.exists() {
            return Ok(MetadataFile::default());
        }

        let text = fs::read_to_string(&path)
            .with_context(|| format!("读取索引失败: {}", path.display()))?;
        serde_json::from_str(&text).with_context(|| format!("解析索引失败: {}", path.display()))
    }

    fn save_metadata(&self, metadata: &MetadataFile) -> Result<()> {
        self.ensure_storage()?;
        let path = self.metadata_path();
        let tmp = path.with_extension("json.tmp");
        let text = serde_json::to_string_pretty(metadata)?;
        fs::write(&tmp, text).with_context(|| format!("写入索引失败: {}", tmp.display()))?;
        fs::rename(&tmp, &path).or_else(|_| {
            fs::copy(&tmp, &path)?;
            fs::remove_file(&tmp)
        })?;
        Ok(())
    }

    fn category_dir(&self, category: &str) -> PathBuf {
        if category.trim().is_empty() {
            self.notes_dir.clone()
        } else {
            self.notes_dir.join(category.trim())
        }
    }

    fn note_path_for(&self, file_name: &str, category: &str) -> PathBuf {
        self.category_dir(category).join(file_name)
    }

    fn note_path(&self, metadata: &NoteMetadata) -> PathBuf {
        self.note_path_for(&metadata.file_name, &metadata.category)
    }

    pub fn list_notes(&self) -> Result<Vec<NoteMetadata>> {
        let mut metadata = self.load_metadata()?;
        metadata.notes.retain(|note| self.note_path(note).is_file());
        metadata
            .notes
            .sort_by_key(|note| std::cmp::Reverse(note.updated_at));
        self.save_metadata(&metadata)?;
        Ok(metadata.notes)
    }

    pub fn list_categories(&self) -> Result<Vec<String>> {
        let metadata = self.load_metadata()?;
        let mut categories = BTreeSet::new();
        for category in metadata.categories {
            let trimmed = category.trim();
            if !trimmed.is_empty() {
                categories.insert(trimmed.to_string());
            }
        }
        for note in metadata.notes {
            let trimmed = note.category.trim();
            if !trimmed.is_empty() {
                categories.insert(trimmed.to_string());
            }
        }
        Ok(categories.into_iter().collect())
    }

    pub fn read_note(&self, id: &str) -> Result<Note> {
        let metadata = self.load_metadata()?;
        let item = metadata
            .notes
            .iter()
            .find(|note| note.id == id)
            .ok_or_else(|| anyhow!("找不到笔记: {id}"))?;
        let content = read_markdown(&self.note_path(item))?;
        Ok(Note {
            id: item.id.clone(),
            title: item.title.clone(),
            file_name: item.file_name.clone(),
            category: item.category.clone(),
            created_at: item.created_at,
            updated_at: item.updated_at,
            word_count: item.word_count,
            content,
        })
    }

    pub fn create_note(&self, request: SaveNoteRequest) -> Result<Note> {
        let mut metadata = self.load_metadata()?;
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let title = normalize_note_title(&request.title, &request.content);
        let file_name = unique_note_file_name(&title, &id);
        let category = request.category.trim().to_string();
        let note_metadata = NoteMetadata {
            id: id.clone(),
            title: title.clone(),
            file_name: file_name.clone(),
            category: category.clone(),
            created_at: now,
            updated_at: now,
            word_count: count_note_chars(&request.content),
            preview: build_preview(&request.content),
        };

        write_markdown(
            &self.note_path_for(&file_name, &category),
            request.content.as_str(),
        )?;
        if !category.is_empty() && !metadata.categories.iter().any(|item| item == &category) {
            metadata.categories.push(category.clone());
        }
        metadata.notes.insert(0, note_metadata.clone());
        self.save_metadata(&metadata)?;

        Ok(Note {
            id,
            title,
            file_name,
            category,
            created_at: now,
            updated_at: now,
            word_count: note_metadata.word_count,
            content: request.content,
        })
    }

    pub fn update_note(&self, id: &str, request: SaveNoteRequest) -> Result<Note> {
        let mut metadata = self.load_metadata()?;
        let index = metadata
            .notes
            .iter()
            .position(|note| note.id == id)
            .ok_or_else(|| anyhow!("找不到笔记: {id}"))?;
        let old = metadata.notes[index].clone();
        let category = request.category.trim().to_string();
        let title = normalize_note_title(&request.title, &request.content);
        let updated_at = Utc::now();
        let old_path = self.note_path(&old);
        let new_path = self.note_path_for(&old.file_name, &category);
        if old_path != new_path && old_path.exists() {
            if let Some(parent) = new_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::rename(&old_path, &new_path).or_else(|_| {
                fs::copy(&old_path, &new_path)?;
                fs::remove_file(&old_path)
            })?;
        }

        write_markdown(&new_path, request.content.as_str())?;
        metadata.notes[index] = NoteMetadata {
            id: old.id.clone(),
            title: title.clone(),
            file_name: old.file_name.clone(),
            category: category.clone(),
            created_at: old.created_at,
            updated_at,
            word_count: count_note_chars(&request.content),
            preview: build_preview(&request.content),
        };
        if !category.is_empty() && !metadata.categories.iter().any(|item| item == &category) {
            metadata.categories.push(category.clone());
        }
        self.save_metadata(&metadata)?;

        Ok(Note {
            id: old.id,
            title,
            file_name: old.file_name,
            category,
            created_at: old.created_at,
            updated_at,
            word_count: count_note_chars(&request.content),
            content: request.content,
        })
    }

    pub fn delete_note(&self, id: &str) -> Result<()> {
        let mut metadata = self.load_metadata()?;
        let index = metadata
            .notes
            .iter()
            .position(|note| note.id == id)
            .ok_or_else(|| anyhow!("找不到笔记: {id}"))?;
        let note = metadata.notes.remove(index);
        let path = self.note_path(&note);
        if path.exists() {
            fs::remove_file(&path)
                .with_context(|| format!("删除笔记文件失败: {}", path.display()))?;
        }
        self.save_metadata(&metadata)
    }

    pub fn import_markdown_file(&self, path: &Path, category: &str) -> Result<Note> {
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        if !matches!(extension.as_str(), "md" | "markdown" | "txt") {
            return Err(anyhow!("只支持导入 Markdown 文件"));
        }
        let content = read_markdown(path)?;
        let title = path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_string();
        self.create_note(SaveNoteRequest {
            title,
            content,
            category: category.to_string(),
        })
    }

    pub fn export_markdown_file(&self, id: &str, path: &Path) -> Result<()> {
        let note = self.read_note(id)?;
        write_markdown(path, note.content.as_str())
    }

    pub fn create_category(&self, name: &str) -> Result<()> {
        let name = normalize_category(name)?;
        let mut metadata = self.load_metadata()?;
        if !metadata.categories.iter().any(|category| category == &name) {
            metadata.categories.push(name.clone());
            metadata.categories.sort();
        }
        fs::create_dir_all(self.category_dir(&name))?;
        self.save_metadata(&metadata)
    }

    pub fn rename_category(&self, old_name: &str, new_name: &str) -> Result<()> {
        let old_name = old_name.trim().to_string();
        if old_name.is_empty() {
            return Err(anyhow!("不能重命名「未分类」"));
        }
        let new_name = normalize_category(new_name)?;
        if new_name == old_name {
            return Ok(());
        }

        let mut metadata = self.load_metadata()?;

        // Update category list
        if let Some(pos) = metadata.categories.iter().position(|c| c == &old_name) {
            metadata.categories[pos] = new_name.clone();
        }
        metadata.categories.sort();

        // Update all notes in this category
        for note in &mut metadata.notes {
            if note.category == old_name {
                note.category = new_name.clone();
            }
        }

        // Move files
        let old_dir = self.category_dir(&old_name);
        let new_dir = self.category_dir(&new_name);
        if old_dir.exists() && old_dir != new_dir {
            fs::create_dir_all(&new_dir)?;
            for entry in fs::read_dir(&old_dir)? {
                let entry = entry?;
                let dest = new_dir.join(entry.file_name());
                fs::rename(entry.path(), &dest)?;
            }
            if old_dir.read_dir()?.next().is_none() {
                let _ = fs::remove_dir(&old_dir);
            }
        }

        self.save_metadata(&metadata)
    }

    pub fn delete_category(&self, name: &str) -> Result<()> {
        let name = name.trim().to_string();
        if name.is_empty() {
            return Err(anyhow!("不能删除「未分类」"));
        }

        let mut metadata = self.load_metadata()?;
        metadata.categories.retain(|c| c != &name);

        // Move notes out of the deleted category
        for note in &mut metadata.notes {
            if note.category == name {
                note.category = String::new();
            }
        }

        // Remove category directory if empty
        let dir = self.category_dir(&name);
        if dir.exists() && dir != self.notes_dir {
            let _ = fs::remove_dir_all(&dir);
        }

        self.save_metadata(&metadata)
    }

    pub fn move_note_to_category(&self, id: &str, category: &str) -> Result<NoteMetadata> {
        let current = self.read_note(id)?;
        let note = self.update_note(
            id,
            SaveNoteRequest {
                title: current.title,
                content: current.content,
                category: category.trim().to_string(),
            },
        )?;
        Ok(metadata_from_note(&note))
    }
}

pub fn metadata_from_note(note: &Note) -> NoteMetadata {
    NoteMetadata {
        id: note.id.clone(),
        title: note.title.clone(),
        file_name: note.file_name.clone(),
        category: note.category.clone(),
        created_at: note.created_at,
        updated_at: note.updated_at,
        word_count: note.word_count,
        preview: build_preview(&note.content),
    }
}

pub fn normalize_note_title(title: &str, content: &str) -> String {
    let title = title.trim();
    if !title.is_empty() {
        return title.chars().take(80).collect();
    }

    content
        .lines()
        .map(str::trim)
        .find_map(|line| {
            let stripped = line.trim_start_matches('#').trim();
            if stripped.is_empty() {
                None
            } else {
                Some(stripped.chars().take(80).collect())
            }
        })
        .unwrap_or_else(|| "未命名笔记".to_string())
}

pub fn build_preview(content: &str) -> String {
    content
        .split_whitespace()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(80)
        .collect()
}

pub fn count_note_chars(content: &str) -> usize {
    content.chars().filter(|ch| !ch.is_whitespace()).count()
}

pub fn format_short_date(value: DateTime<Utc>) -> String {
    value.with_timezone(&Local).format("%m-%d").to_string()
}

pub fn format_time(value: DateTime<Utc>) -> String {
    value.with_timezone(&Local).format("%H:%M").to_string()
}

pub fn format_full_time(value: DateTime<Utc>) -> String {
    value
        .with_timezone(&Local)
        .format("%m-%d %H:%M")
        .to_string()
}

fn unique_note_file_name(title: &str, id: &str) -> String {
    let stem = safe_file_stem(title).unwrap_or_else(|| "untitled".to_string());
    let suffix: String = id.chars().take(8).collect();
    format!("{stem}-{suffix}.md")
}

fn safe_file_stem(value: &str) -> Option<String> {
    let sanitized: String = value
        .trim()
        .chars()
        .map(|ch| {
            if matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') || ch.is_control()
            {
                '_'
            } else {
                ch
            }
        })
        .collect();
    let normalized = sanitized
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("_")
        .trim_matches('_')
        .chars()
        .take(80)
        .collect::<String>();
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

fn normalize_category(name: &str) -> Result<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("分类名不能为空"));
    }
    if trimmed.chars().any(|ch| {
        matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') || ch.is_control()
    }) {
        return Err(anyhow!("分类名不能包含特殊字符"));
    }
    Ok(trimmed.to_string())
}
