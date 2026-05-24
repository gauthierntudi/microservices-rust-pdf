use std::env;

pub struct AppConfig {
    pub port: u16,
    pub api_key: String,
    pub max_upload_bytes: usize,
    pub max_pages: u32,
    pub default_dpi: u32,
    pub pdftoppm_bin: String,
}

impl AppConfig {
    pub fn from_env() -> Self {
        let max_upload_mb: u64 = env::var("PDF_MAX_UPLOAD_MB")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(50);

        Self {
            port: env::var("PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(8080),
            api_key: env::var("PDF_SERVICE_API_KEY")
                .unwrap_or_else(|_| "change-me-in-production".into()),
            max_upload_bytes: (max_upload_mb.saturating_mul(1024 * 1024)) as usize,
            max_pages: env::var("PDF_MAX_PAGES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(100),
            default_dpi: env::var("PDF_DEFAULT_DPI")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(150),
            pdftoppm_bin: env::var("PDFTOPPM_BIN").unwrap_or_else(|_| "pdftoppm".into()),
        }
    }
}
