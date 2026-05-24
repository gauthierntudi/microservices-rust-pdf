use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::Serialize;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};
use uuid::Uuid;

#[derive(Clone, Copy)]
pub struct RasterizeOptions {
    pub dpi: u32,
    pub max_pages: u32,
}

#[derive(Serialize)]
pub struct RasterizedPage {
    pub page_number: u32,
    pub width: u32,
    pub height: u32,
    pub mime: &'static str,
    pub data_base64: String,
}

#[derive(Serialize)]
pub struct ResponseMeta {
    pub dpi: u32,
    pub processing_ms: u64,
}

#[derive(Serialize)]
pub struct RasterizeResponse {
    pub status: &'static str,
    pub page_count: usize,
    pub pages: Vec<RasterizedPage>,
    pub meta: ResponseMeta,
}

#[derive(Debug)]
pub struct RasterizeError {
    message: String,
}

impl std::fmt::Display for RasterizeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for RasterizeError {}

impl RasterizeError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

pub fn rasterize_pdf(
    pdf_bytes: &[u8],
    options: &RasterizeOptions,
    pdftoppm_bin: &str,
) -> Result<Vec<RasterizedPage>, RasterizeError> {
  if !pdf_bytes.starts_with(b"%PDF") {
        return Err(RasterizeError::new("Fichier invalide : en-tête PDF attendu."));
    }

    let work_dir = std::env::temp_dir().join(format!("authentiq-pdf-{}", Uuid::new_v4()));
    fs::create_dir_all(&work_dir).map_err(|e| RasterizeError::new(format!("Temp dir: {e}")))?;

    let result = rasterize_in_dir(pdf_bytes, options, pdftoppm_bin, &work_dir);
    let _ = fs::remove_dir_all(&work_dir);
    result
}

fn rasterize_in_dir(
    pdf_bytes: &[u8],
    options: &RasterizeOptions,
    pdftoppm_bin: &str,
    work_dir: &Path,
) -> Result<Vec<RasterizedPage>, RasterizeError> {
    let input_path = work_dir.join("input.pdf");
    let output_prefix = work_dir.join("page");

    {
        let mut file = fs::File::create(&input_path)
            .map_err(|e| RasterizeError::new(format!("Écriture PDF temporaire: {e}")))?;
        file.write_all(pdf_bytes)
            .map_err(|e| RasterizeError::new(format!("Écriture PDF temporaire: {e}")))?;
    }

    let status = Command::new(pdftoppm_bin)
        .arg("-jpeg")
        .arg("-r")
        .arg(options.dpi.to_string())
        .arg("-f")
        .arg("1")
        .arg("-l")
        .arg(options.max_pages.to_string())
        .arg(&input_path)
        .arg(&output_prefix)
        .status()
        .map_err(|e| {
            RasterizeError::new(format!(
                "Impossible d'exécuter {pdftoppm_bin} (poppler-utils requis): {e}"
            ))
        })?;

    if !status.success() {
        return Err(RasterizeError::new(format!(
            "{pdftoppm_bin} a échoué (code {:?})",
            status.code()
        )));
    }

    let mut paths: Vec<PathBuf> = fs::read_dir(work_dir)
        .map_err(|e| RasterizeError::new(format!("Lecture sortie: {e}")))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("jpg"))
                .unwrap_or(false)
        })
        .collect();

    paths.sort_by(|a, b| page_index_from_path(a).cmp(&page_index_from_path(b)));

    if paths.is_empty() {
        return Err(RasterizeError::new(
            "Aucune page extraite du PDF (fichier vide ou protégé ?).",
        ));
    }

    let mut pages = Vec::with_capacity(paths.len());

    for (index, path) in paths.into_iter().enumerate() {
        let bytes = fs::read(&path)
            .map_err(|e| RasterizeError::new(format!("Lecture page JPEG: {e}")))?;
        let (width, height) = jpeg_dimensions(&bytes).unwrap_or((0, 0));

        pages.push(RasterizedPage {
            page_number: (index + 1) as u32,
            width,
            height,
            mime: "image/jpeg",
            data_base64: STANDARD.encode(bytes),
        });
    }

    Ok(pages)
}

fn page_index_from_path(path: &Path) -> u32 {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .and_then(|stem| stem.rsplit('-').next())
        .and_then(|n| n.parse().ok())
        .unwrap_or(0)
}

fn jpeg_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 4 || bytes[0] != 0xFF || bytes[1] != 0xD8 {
        return None;
    }

    let mut i = 2usize;
    while i + 9 < bytes.len() {
        if bytes[i] != 0xFF {
            i += 1;
            continue;
        }

        let marker = bytes[i + 1];
        if marker == 0xC0 || marker == 0xC2 {
            let height = u16::from_be_bytes([bytes[i + 5], bytes[i + 6]]) as u32;
            let width = u16::from_be_bytes([bytes[i + 7], bytes[i + 8]]) as u32;
            return Some((width, height));
        }

        if i + 3 >= bytes.len() {
            break;
        }
        let segment_len = u16::from_be_bytes([bytes[i + 2], bytes[i + 3]]) as usize;
        if segment_len < 2 {
            break;
        }
        i += segment_len + 2;
    }

    None
}
