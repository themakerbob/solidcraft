//! SolidCraft drawing sidecar format (.scdraw), version 1.
//! This module is deliberately independent of the egui drawing workspace.
//! Runtime HLR, textures and selection state must never be serialized.
use serde::{Deserialize, Serialize};
use std::{fs, io, path::{Path, PathBuf}};

pub const FORMAT: &str = "solidcraft-drawing";
pub const VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DrawingFile {
    pub format: String,
    pub version: u32,
    pub model: ModelReference,
    #[serde(default)]
    pub sheets: Vec<Sheet>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelReference {
    /// Path relative to the .scdraw file directory whenever possible.
    pub path: PathBuf,
    /// Optional until the .sc document has a persistent identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document_id: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Sheet {
    pub name: String,
    pub paper: String,
    pub landscape: bool,
    pub first_angle: bool,
    pub border: bool,
    pub title_block: TitleBlock,
    #[serde(default)] pub views: Vec<View>,
    #[serde(default)] pub annotations: Vec<Annotation>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct TitleBlock {
    pub title: String,
    pub drawing_number: String,
    pub revision: String,
    pub project: String,
    pub organisation: String,
    pub drawn_by: String,
    pub checked_by: String,
    pub date: String,
    pub material: String,
    pub finish: String,
    pub general_tolerances: String,
    #[serde(default)] pub revisions: Vec<Revision>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Revision { pub revision: String, pub description: String, pub date: String, pub approved_by: String }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct View {
    pub id: u64,
    pub orientation: String,
    pub x_mm: f64,
    pub y_mm: f64,
    pub scale: f64,
    pub display: String,
    pub label: String,
    pub visible: bool,
    pub parent: Option<u64>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Annotation {
    Note { x_mm: f64, y_mm: f64, text: String },
    Leader { target: [f64; 2], text_at: [f64; 2], text: String },
    Dimension { a: [f64; 2], b: [f64; 2], mode: String, offset: f64 },
    Radius { view_id: u64, center: [f64; 2], radius: f64, diameter: bool, angle_rad: f64, text_shift: [f64; 2], tolerance: Tolerance },
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Tolerance { pub mode: String, pub upper: f64, pub lower: f64, pub precision: usize }
impl DrawingFile {
    pub fn new(model_path: impl Into<PathBuf>) -> Self {
        Self { format: FORMAT.into(), version: VERSION, model: ModelReference { path: model_path.into(), document_id: None }, sheets: Vec::new() }
    }
    pub fn validate(&self) -> io::Result<()> {
        if self.format != FORMAT || self.version != VERSION {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Unsupported .scdraw format or version"));
        }
        if self.model.path.as_os_str().is_empty() {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Drawing has no model reference"));
        }
        Ok(())
    }
    pub fn model_path(&self, drawing_path: &Path) -> PathBuf {
        if self.model.path.is_absolute() { self.model.path.clone() }
        else { drawing_path.parent().unwrap_or_else(|| Path::new(".")).join(&self.model.path) }
    }
}
/// Atomic replacement on Windows needs the destination removed before rename;
/// retain a backup on failure rather than silently truncating a valid drawing.
pub fn save(path: &Path, document: &DrawingFile) -> io::Result<()> {
    document.validate()?;
    let bytes = serde_json::to_vec_pretty(document).map_err(io::Error::other)?;
    let temp = path.with_extension("scdraw.tmp");
    fs::write(&temp, bytes)?;
    if path.exists() {
        let backup = path.with_extension("scdraw.bak");
        if backup.exists() { fs::remove_file(&backup)?; }
        fs::rename(path, &backup)?;
        match fs::rename(&temp, path) {
            Ok(()) => { let _ = fs::remove_file(backup); Ok(()) },
            Err(e) => { let _ = fs::rename(backup, path); Err(e) }
        }
    } else { fs::rename(temp, path) }
}
pub fn load(path: &Path) -> io::Result<DrawingFile> {
    let data = fs::read(path)?;
    let document: DrawingFile = serde_json::from_slice(&data).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    document.validate()?;
    Ok(document)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn round_trip() {
        let mut d = DrawingFile::new("Bracket.sc");
        d.sheets.push(Sheet { name: "Sheet 1".into(), paper: "A3".into(), landscape: true, first_angle: true, border: true, title_block: TitleBlock { title: "Bracket".into(), ..Default::default() }, views: vec![], annotations: vec![Annotation::Note { x_mm: 10.0, y_mm: 20.0, text: "Deburr".into() }] });
        let encoded = serde_json::to_string(&d).unwrap();
        let decoded: DrawingFile = serde_json::from_str(&encoded).unwrap();
        assert_eq!(d, decoded);
        assert_eq!(decoded.model_path(Path::new("/project/Bracket.scdraw")), Path::new("/project/Bracket.sc"));
    }
    #[test]
    fn reject_unknown_version() {
        let mut d = DrawingFile::new("x.sc"); d.version = 99;
        assert!(d.validate().is_err());
    }
}
