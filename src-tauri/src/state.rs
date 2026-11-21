use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    fs, io,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

const MAX_COUNTER: i64 = 9_007_199_254_740_991;
const COLORS: [&str; 4] = ["honey", "lilac", "mint", "rose"];

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct Frame {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Default, Deserialize, Serialize)]
pub struct Settings {
    #[serde(default)]
    pub pinned: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bounds: Option<Frame>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame: Option<Frame>,
}

#[derive(Clone, Deserialize, Serialize)]
struct Note {
    id: String,
    title: String,
    text: String,
    color: String,
}

impl Note {
    fn new(title: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            title: title.into(),
            text: String::new(),
            color: COLORS[0].into(),
        }
    }
}

#[derive(Clone, Deserialize, Serialize)]
struct Reset {
    value: i64,
    at: u64,
}

#[derive(Clone, Default, Deserialize, Serialize)]
struct Counter {
    value: i64,
    history: Vec<Reset>,
}

#[derive(Clone, Deserialize, Serialize)]
struct Template {
    id: String,
    title: String,
    text: String,
}

#[derive(Clone, Deserialize, Serialize)]
struct Data {
    schema: u8,
    notes: Vec<Note>,
    counter: Counter,
    templates: Vec<Template>,
    window: Settings,
}

impl Default for Data {
    fn default() -> Self {
        Self {
            schema: 1,
            notes: vec![Note::new("Первая заметка")],
            counter: Counter::default(),
            templates: Vec::new(),
            window: Settings::default(),
        }
    }
}

#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    #[serde(rename = "note:add")]
    AddNote,
    #[serde(rename = "note:edit")]
    EditNote {
        id: String,
        field: String,
        value: String,
    },
    #[serde(rename = "note:delete")]
    DeleteNote { id: String },
    #[serde(rename = "counter:step")]
    Step { step: i64 },
    #[serde(rename = "counter:reset")]
    Reset,
    #[serde(rename = "template:save")]
    SaveTemplate {
        id: Option<String>,
        title: String,
        text: String,
    },
    #[serde(rename = "template:delete")]
    DeleteTemplate { id: String },
    #[serde(rename = "template:move")]
    MoveTemplate {
        id: String,
        target: String,
        after: bool,
    },
}

pub struct Store {
    path: PathBuf,
    revision: u64,
    data: Data,
}

fn checked_text(value: &str, limit: usize) -> Result<(), String> {
    if value.encode_utf16().count() > limit {
        return Err("Текст превышает допустимую длину.".into());
    }
    Ok(())
}

fn missing() -> String {
    "Запись удалена.".into()
}

impl Store {
    pub fn open(path: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let directory = path
            .parent()
            .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?;
        fs::create_dir_all(directory)?;
        let data: Data = match fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes)?,
            Err(error) if error.kind() == io::ErrorKind::NotFound => Data::default(),
            Err(error) => return Err(error.into()),
        };
        if data.schema != 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Неизвестный формат данных AnyNotes.",
            )
            .into());
        }
        Ok(Self {
            path,
            data,
            revision: 0,
        })
    }

    fn commit<T>(
        &mut self,
        update: impl FnOnce(&mut Data) -> Result<T, String>,
    ) -> Result<T, String> {
        let mut next = self.data.clone();
        let result = update(&mut next)?;
        let bytes = serde_json::to_vec(&next).map_err(|error| error.to_string())?;
        let temporary = self.path.with_extension("json.tmp");
        write_private(&temporary, &bytes).map_err(|error| error.to_string())?;
        fs::rename(temporary, &self.path).map_err(|error| error.to_string())?;
        self.data = next;
        self.revision += 1;
        Ok(result)
    }

    pub fn settings(&self) -> &Settings {
        &self.data.window
    }

    pub fn save_frame(&mut self, frame: Frame) -> Result<(), String> {
        if self.settings().frame == Some(frame) {
            return Ok(());
        }
        self.commit(|data| {
            data.window.frame = Some(frame);
            Ok(())
        })
    }

    pub fn save_pin(&mut self, pinned: bool) -> Result<(), String> {
        self.commit(|data| {
            data.window.pinned = pinned;
            Ok(())
        })
    }

    pub fn snapshot(&self) -> Value {
        json!({
            "revision": self.revision,
            "notes": self.data.notes,
            "counter": self.data.counter,
            "templates": self.data.templates,
        })
    }

    pub fn text_to_copy(&self, id: &str) -> Result<&str, String> {
        self.data
            .templates
            .iter()
            .find(|item| item.id == id)
            .map(|item| item.text.as_str())
            .ok_or_else(missing)
    }

    pub fn apply(&mut self, action: Action) -> Result<Value, String> {
        let result = self.commit(|data| {
            match action {
                Action::AddNote => {
                    let note = Note::new("");
                    let id = note.id.clone();
                    data.notes.insert(0, note);
                    return Ok(json!(id));
                }
                Action::EditNote { id, field, value } => {
                    let note = data
                        .notes
                        .iter_mut()
                        .find(|item| item.id == id)
                        .ok_or_else(missing)?;
                    match field.as_str() {
                        "title" => {
                            checked_text(&value, 120)?;
                            note.title = value;
                        }
                        "text" => {
                            checked_text(&value, 100_000)?;
                            note.text = value;
                        }
                        "color" if COLORS.contains(&value.as_str()) => note.color = value,
                        _ => return Err("Неизвестное свойство заметки.".into()),
                    }
                }
                Action::DeleteNote { id } => data.notes.retain(|note| note.id != id),
                Action::Step { step } => {
                    if step != 1 && step != -1 {
                        return Err("Некорректный шаг счётчика.".into());
                    }
                    data.counter.value = data
                        .counter
                        .value
                        .saturating_add(step)
                        .clamp(-MAX_COUNTER, MAX_COUNTER);
                }
                Action::Reset => {
                    let at = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64;
                    data.counter.history.insert(
                        0,
                        Reset {
                            value: data.counter.value,
                            at,
                        },
                    );
                    data.counter.history.truncate(10);
                    data.counter.value = 0;
                }
                Action::SaveTemplate { id, title, text } => {
                    checked_text(&title, 120)?;
                    checked_text(&text, 100_000)?;
                    let title = title.trim().to_owned();
                    if title.is_empty() || text.trim().is_empty() {
                        return Err("Укажите название и текст шаблона.".into());
                    }
                    if let Some(id) = id {
                        let item = data
                            .templates
                            .iter_mut()
                            .find(|item| item.id == id)
                            .ok_or_else(missing)?;
                        item.title = title;
                        item.text = text;
                    } else {
                        let id = Uuid::new_v4().to_string();
                        data.templates.insert(0, Template { id, title, text });
                    }
                }
                Action::DeleteTemplate { id } => data.templates.retain(|item| item.id != id),
                Action::MoveTemplate { id, target, after } => {
                    let from = data
                        .templates
                        .iter()
                        .position(|item| item.id == id)
                        .ok_or_else(missing)?;
                    if id != target {
                        let item = data.templates.remove(from);
                        let to = data
                            .templates
                            .iter()
                            .position(|item| item.id == target)
                            .ok_or_else(missing)?;
                        data.templates.insert(to + usize::from(after), item);
                    }
                }
            }
            Ok(Value::Null)
        })?;
        Ok(json!({ "result": result, "data": self.snapshot() }))
    }
}

fn write_private(path: &std::path::Path, bytes: &[u8]) -> io::Result<()> {
    use std::io::Write;
    let mut options = fs::OpenOptions::new();
    options.create(true).truncate(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)?.write_all(bytes)
}
