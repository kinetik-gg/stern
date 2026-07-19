use std::fmt::Write as _;

#[derive(Clone)]
pub(super) enum Json {
    Null,
    Bool(bool),
    Number(String),
    String(String),
    Array(Vec<Self>),
    Object(Vec<(String, Self)>),
}

impl Json {
    pub(super) fn to_pretty_bytes(&self) -> Vec<u8> {
        let mut output = String::new();
        self.write(&mut output, 0);
        output.push('\n');
        output.into_bytes()
    }

    pub(super) fn bool_field(&self, key: &str) -> Option<bool> {
        let Self::Object(fields) = self else {
            return None;
        };
        let (_, value) = fields.iter().find(|(name, _)| name == key)?;
        match value {
            Self::Bool(value) => Some(*value),
            _ => None,
        }
    }

    fn write(&self, output: &mut String, depth: usize) {
        match self {
            Self::Null => output.push_str("null"),
            Self::Bool(value) => output.push_str(if *value { "true" } else { "false" }),
            Self::Number(value) => output.push_str(value),
            Self::String(value) => write_string(output, value),
            Self::Array(values) => write_array(output, values, depth),
            Self::Object(fields) => write_object(output, fields, depth),
        }
    }
}

fn write_array(output: &mut String, values: &[Json], depth: usize) {
    if values.is_empty() {
        output.push_str("[]");
        return;
    }
    output.push_str("[\n");
    for (index, value) in values.iter().enumerate() {
        indent(output, depth + 1);
        value.write(output, depth + 1);
        output.push_str(if index + 1 == values.len() {
            "\n"
        } else {
            ",\n"
        });
    }
    indent(output, depth);
    output.push(']');
}

fn write_object(output: &mut String, fields: &[(String, Json)], depth: usize) {
    if fields.is_empty() {
        output.push_str("{}");
        return;
    }
    output.push_str("{\n");
    for (index, (key, value)) in fields.iter().enumerate() {
        indent(output, depth + 1);
        write_string(output, key);
        output.push_str(": ");
        value.write(output, depth + 1);
        output.push_str(if index + 1 == fields.len() {
            "\n"
        } else {
            ",\n"
        });
    }
    indent(output, depth);
    output.push('}');
}

fn indent(output: &mut String, depth: usize) {
    for _ in 0..depth {
        output.push_str("  ");
    }
}

fn write_string(output: &mut String, value: &str) {
    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            value if value.is_control() => {
                let _ = write!(output, "\\u{:04x}", u32::from(value));
            }
            value => output.push(value),
        }
    }
    output.push('"');
}

macro_rules! json {
    ({ $($key:literal : $value:expr),* $(,)? }) => {
        $crate::json::Json::Object(vec![$(($key.to_owned(), $crate::json::Json::from($value))),*])
    };
    ($value:expr) => { $crate::json::Json::from($value) };
}
pub(super) use json;

impl From<bool> for Json {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}
impl From<String> for Json {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}
impl From<&String> for Json {
    fn from(value: &String) -> Self {
        Self::String(value.clone())
    }
}
impl From<&str> for Json {
    fn from(value: &str) -> Self {
        Self::String(value.to_owned())
    }
}
impl From<&&str> for Json {
    fn from(value: &&str) -> Self {
        Self::String((*value).to_owned())
    }
}
impl<T: Into<Json>> From<Option<T>> for Json {
    fn from(value: Option<T>) -> Self {
        value.map_or(Self::Null, Into::into)
    }
}
impl<T: Into<Json>> From<Vec<T>> for Json {
    fn from(value: Vec<T>) -> Self {
        Self::Array(value.into_iter().map(Into::into).collect())
    }
}
impl<T: Into<Json>, const N: usize> From<[T; N]> for Json {
    fn from(value: [T; N]) -> Self {
        Self::Array(value.into_iter().map(Into::into).collect())
    }
}
impl<T: Clone + Into<Json>> From<&[T]> for Json {
    fn from(value: &[T]) -> Self {
        Self::Array(value.iter().cloned().map(Into::into).collect())
    }
}

macro_rules! number {
    ($($kind:ty),* $(,)?) => {$(
        impl From<$kind> for Json { fn from(value: $kind) -> Self { Self::Number(value.to_string()) } }
    )*};
}
number!(i32, i64, u8, u32, u64, usize, f32, f64);
