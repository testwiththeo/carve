use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Supported vector annotation tool kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VectorToolKind {
    #[serde(alias = "Arrow", alias = "arrow")]
    Arrow,
    #[serde(alias = "Rectangle", alias = "rectangle")]
    Rectangle,
    #[serde(alias = "Ellipse", alias = "ellipse")]
    Ellipse,
    #[serde(alias = "Pencil", alias = "pencil")]
    Pencil,
    #[serde(
        alias = "CounterBadge",
        alias = "counterBadge",
        alias = "counter_badge"
    )]
    CounterBadge,
    #[serde(alias = "Text", alias = "text")]
    Text,
    #[serde(alias = "Blur", alias = "blur")]
    Blur,
    #[serde(alias = "Highlight", alias = "highlight")]
    Highlight,
}

impl VectorToolKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Arrow => "arrow",
            Self::Rectangle => "rectangle",
            Self::Ellipse => "ellipse",
            Self::Pencil => "pencil",
            Self::CounterBadge => "counter_badge",
            Self::Text => "text",
            Self::Blur => "blur",
            Self::Highlight => "highlight",
        }
    }
}

impl fmt::Display for VectorToolKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for VectorToolKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "arrow" => Ok(Self::Arrow),
            "rectangle" | "rect" => Ok(Self::Rectangle),
            "ellipse" | "circle" => Ok(Self::Ellipse),
            "pencil" | "freehand" => Ok(Self::Pencil),
            "counter_badge" | "counterbadge" | "counter" | "step" => Ok(Self::CounterBadge),
            "text" => Ok(Self::Text),
            "blur" | "pixelate" => Ok(Self::Blur),
            "highlight" | "highlighter" => Ok(Self::Highlight),
            other => Err(format!("Unknown vector tool kind: {}", other)),
        }
    }
}

/// An individual vector annotation element positioned along the recording timeline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VectorElement {
    pub id: String,
    pub tool: VectorToolKind,
    pub start_ms: u64,
    pub end_ms: u64,
    #[serde(default)]
    pub data: serde_json::Value,
}

impl VectorElement {
    pub fn new(
        id: impl Into<String>,
        tool: VectorToolKind,
        start_ms: u64,
        end_ms: u64,
        data: serde_json::Value,
    ) -> Self {
        Self {
            id: id.into(),
            tool,
            start_ms,
            end_ms,
            data,
        }
    }

    /// Returns true if this element is active at the given millisecond timestamp (inclusive).
    pub fn is_active_at(&self, timestamp_ms: u64) -> bool {
        timestamp_ms >= self.start_ms && timestamp_ms <= self.end_ms
    }
}

/// In-memory ledger holding timed vector annotations for a recording session.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TimedVectorLedger {
    #[serde(default)]
    pub elements: Vec<VectorElement>,
    #[serde(default)]
    next_id: u64,
}

impl TimedVectorLedger {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an element to the ledger.
    pub fn add_element(&mut self, element: VectorElement) {
        self.elements.push(element);
    }

    /// Remove an element by its unique ID.
    pub fn remove_element(&mut self, id: &str) -> Option<VectorElement> {
        if let Some(pos) = self.elements.iter().position(|e| e.id == id) {
            Some(self.elements.remove(pos))
        } else {
            None
        }
    }

    /// Clear all elements from the ledger.
    pub fn clear(&mut self) {
        self.elements.clear();
    }

    /// Query elements that are active at the given millisecond timestamp.
    pub fn active_at(&self, timestamp_ms: u64) -> Vec<&VectorElement> {
        self.elements
            .iter()
            .filter(|e| e.is_active_at(timestamp_ms))
            .collect()
    }

    /// Query elements that are active at the given millisecond timestamp, returning cloned instances.
    pub fn active_elements_at(&self, timestamp_ms: u64) -> Vec<VectorElement> {
        self.elements
            .iter()
            .filter(|e| e.is_active_at(timestamp_ms))
            .cloned()
            .collect()
    }

    /// Reference to all elements in the ledger.
    pub fn elements(&self) -> &[VectorElement] {
        &self.elements
    }

    /// Mutable reference to all elements in the ledger.
    pub fn elements_mut(&mut self) -> &mut [VectorElement] {
        &mut self.elements
    }

    /// Find an element by ID.
    pub fn get_element(&self, id: &str) -> Option<&VectorElement> {
        self.elements.iter().find(|e| e.id == id)
    }

    /// Find an element by ID with mutable access.
    pub fn get_element_mut(&mut self, id: &str) -> Option<&mut VectorElement> {
        self.elements.iter_mut().find(|e| e.id == id)
    }

    /// Number of elements in the ledger.
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    /// Check if the ledger is empty.
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    /// Generate a monotonic unique ID with a specified prefix.
    pub fn generate_id(&mut self, prefix: &str) -> String {
        self.next_id += 1;
        format!("{}_{}", prefix, self.next_id)
    }

    /// Generate a monotonic unique ID with default "elem" prefix.
    pub fn next_id(&mut self) -> String {
        self.generate_id("elem")
    }
}

impl From<Vec<VectorElement>> for TimedVectorLedger {
    fn from(elements: Vec<VectorElement>) -> Self {
        Self {
            elements,
            next_id: 0,
        }
    }
}
