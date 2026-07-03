use crate::error::{AlexandriaError, Result};
use crate::models::FileMetadata;
use lopdf::Document;
use std::collections::HashMap;
use std::path::Path;

pub fn extract_pdf_metadata(path: &Path) -> Result<FileMetadata> {
    let doc = Document::load(path).map_err(|e| AlexandriaError::Other(e.into()))?;

    let pages = doc.get_pages().len() as i64;

    let mut extra: HashMap<String, serde_json::Value> = HashMap::new();
    extra.insert("pages".to_string(), pages.into());

    if let Ok(info_id) = doc.trailer.get(b"Info").and_then(|o| o.as_reference()) {
        if let Ok(dict) = doc.get_dictionary(info_id) {
            for (key, value) in dict.iter() {
                let key_str = String::from_utf8_lossy(key).to_string();
                let val_str = value
                    .as_str()
                    .ok()
                    .map(|s| String::from_utf8_lossy(s).to_string())
                    .unwrap_or_else(|| format!("{:?}", value));
                extra.insert(key_str, val_str.into());
            }
        }
    }

    Ok(FileMetadata {
        file_type: "pdf".to_string(),
        extra_json: Some(serde_json::to_string(&extra).unwrap_or_default()),
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::{Dictionary, Object};
    use std::fs;

    fn build_test_pdf_path() -> std::path::PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("alexandria-test-{}.pdf", std::process::id()));
        path
    }

    fn create_test_pdf_with_info(path: &Path) {
        let mut doc = Document::with_version("1.4");

        let pages_id = doc.new_object_id();
        let page_id = doc.add_object({
            let mut d = Dictionary::new();
            d.set("Type", "Page");
            d.set("Parent", pages_id);
            Object::Dictionary(d)
        });

        doc.objects.insert(
            pages_id,
            Object::Dictionary({
                let mut d = Dictionary::new();
                d.set("Type", "Pages");
                d.set("Kids", vec![Object::Reference(page_id)]);
                d.set("Count", 1i64);
                d
            }),
        );

        let catalog_id = doc.add_object({
            let mut d = Dictionary::new();
            d.set("Type", "Catalog");
            d.set("Pages", pages_id);
            Object::Dictionary(d)
        });
        doc.trailer.set("Root", catalog_id);

        let info_id = doc.add_object({
            let mut d = Dictionary::new();
            d.set("Title", Object::string_literal("Test PDF"));
            d.set("Author", Object::string_literal("Alexandria"));
            Object::Dictionary(d)
        });
        doc.trailer.set("Info", info_id);

        doc.save(path).unwrap();
    }

    #[test]
    fn test_extract_pdf_metadata_reads_pages_and_info() {
        let path = build_test_pdf_path();
        create_test_pdf_with_info(&path);

        let meta = extract_pdf_metadata(&path).unwrap();
        assert_eq!(meta.file_type, "pdf");

        let extra: serde_json::Value =
            serde_json::from_str(meta.extra_json.as_ref().unwrap()).unwrap();
        assert_eq!(extra["pages"], 1);
        assert_eq!(extra["Title"], "Test PDF");
        assert_eq!(extra["Author"], "Alexandria");

        fs::remove_file(&path).ok();
    }
}
