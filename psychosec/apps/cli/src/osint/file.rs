use crate::output::{Output, Status};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::Path;

pub async fn metadata(input: &str) -> Result<Output, String> {
    let p = input.trim().trim_matches('"').to_string();
    tokio::task::spawn_blocking(move || inspect(Path::new(&p))).await.map_err(|e| e.to_string())?
}

fn fmt_time(t: std::io::Result<std::time::SystemTime>) -> Option<String> {
    t.ok().map(|t| chrono::DateTime::<chrono::Utc>::from(t).format("%Y-%m-%d %H:%M:%S UTC").to_string())
}

pub fn inspect(path: &Path) -> Result<Output, String> {
    let md = std::fs::metadata(path).map_err(|e| format!("cannot read '{}': {e}", path.display()))?;
    if !md.is_file() {
        return Err(format!("'{}' is not a file", path.display()));
    }
    let mut f = std::fs::File::open(path).map_err(|e| format!("cannot open '{}': {e}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut head = Vec::with_capacity(8192);
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = f.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 { break; }
        if head.len() < 8192 {
            head.extend_from_slice(&buf[..n.min(8192 - head.len())]);
        }
        hasher.update(&buf[..n]);
    }
    let name = path.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let mut o = Output::new("file", &path.display().to_string(), &format!("File metadata: {name}"));
    o.heading("File");
    o.kv("Name", name.clone()).kv("Size", format!("{} bytes", md.len()));
    o.kv("SHA-256", hex::encode(hasher.finalize()));
    if let Some(t) = fmt_time(md.modified()) { o.kv("Modified", t); }
    if let Some(t) = fmt_time(md.created()) { o.kv("Created", t); }
    let kind = infer::get(&head);
    match kind {
        Some(k) => { o.kv("Type", format!("{} ({})", k.mime_type(), k.extension())); }
        None => { o.kv("Type", "unknown / plain data"); }
    }
    if let (Some(k), Some(ext)) = (kind, path.extension().and_then(|e| e.to_str())) {
        let ext = ext.to_ascii_lowercase();
        let same = k.extension() == ext || (k.extension() == "jpg" && ext == "jpeg") || (k.extension() == "zip" && matches!(ext.as_str(), "docx" | "xlsx" | "pptx" | "odt" | "jar" | "apk"));
        if !same {
            o.item(Status::Warn, "Extension", format!("'.{ext}' does not match detected content '.{}'", k.extension()));
        }
    }
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    let mime = kind.map(|k| k.mime_type()).unwrap_or("");
    if mime == "image/jpeg" || mime == "image/tiff" || mime == "image/png" || mime == "image/webp" || mime == "image/heif" {
        exif_section(&mut o, path);
    }
    if mime == "application/pdf" || ext == "pdf" {
        pdf_section(&mut o, path);
    }
    if matches!(ext.as_str(), "docx" | "xlsx" | "pptx") && mime == "application/zip" || matches!(ext.as_str(), "docx" | "xlsx" | "pptx") {
        office_section(&mut o, path);
    }
    Ok(o)
}

fn exif_section(o: &mut Output, path: &Path) {
    o.heading("EXIF");
    let Ok(file) = std::fs::File::open(path) else { return };
    match exif::Reader::new().read_from_container(&mut std::io::BufReader::new(file)) {
        Ok(ex) => {
            let mut n = 0;
            for f in ex.fields() {
                let v = f.display_value().with_unit(&ex).to_string();
                if v.len() > 200 || f.tag.to_string().is_empty() { continue; }
                o.kv(f.tag.to_string(), v);
                n += 1;
            }
            if n == 0 { o.text("No EXIF fields present."); }
        }
        Err(e) => { o.item(Status::Dim, "EXIF", format!("none found ({e})")); }
    }
}

fn pdf_section(o: &mut Output, path: &Path) {
    o.heading("PDF");
    match lopdf::Document::load(path) {
        Ok(doc) => {
            o.kv("Version", doc.version.clone()).kv("Pages", doc.get_pages().len().to_string());
            if doc.is_encrypted() { o.item(Status::Warn, "Encryption", "document is encrypted; some metadata may be unreadable"); }
            let info = doc.trailer.get(b"Info").ok().and_then(|i| match i {
                lopdf::Object::Reference(id) => doc.get_dictionary(*id).ok(),
                lopdf::Object::Dictionary(d) => Some(d),
                _ => None,
            });
            match info {
                Some(d) => {
                    for key in ["Title", "Author", "Subject", "Keywords", "Creator", "Producer", "CreationDate", "ModDate"] {
                        if let Ok(v) = d.get(key.as_bytes()) {
                            if let Ok(s) = v.as_str() {
                                let txt = decode_pdf_string(s);
                                if !txt.trim().is_empty() { o.kv(key, txt); }
                            }
                        }
                    }
                }
                None => { o.text("No document information dictionary."); }
            }
        }
        Err(e) => { o.item(Status::Err, "PDF", format!("cannot parse: {e}")); }
    }
}

pub fn decode_pdf_string(b: &[u8]) -> String {
    if b.starts_with(&[0xFE, 0xFF]) {
        let u: Vec<u16> = b[2..].chunks(2).filter(|c| c.len() == 2).map(|c| u16::from_be_bytes([c[0], c[1]])).collect();
        String::from_utf16_lossy(&u)
    } else {
        String::from_utf8(b.to_vec()).unwrap_or_else(|_| b.iter().map(|&c| c as char).collect())
    }
}

fn office_section(o: &mut Output, path: &Path) {
    o.heading("Office document");
    let Ok(file) = std::fs::File::open(path) else { return };
    let mut zip = match zip::ZipArchive::new(file) {
        Ok(z) => z,
        Err(e) => { o.item(Status::Err, "Office", format!("not a valid OOXML container: {e}")); return; }
    };
    for (part, tags) in [
        ("docProps/core.xml", &["dc:title", "dc:subject", "dc:creator", "cp:lastModifiedBy", "cp:keywords", "dc:description", "dcterms:created", "dcterms:modified", "cp:revision"][..]),
        ("docProps/app.xml", &["Application", "AppVersion", "Company", "Template", "TotalTime", "Pages", "Words"][..]),
    ] {
        let mut s = String::new();
        match zip.by_name(part) {
            Ok(mut f) => { let _ = f.by_ref().take(1 << 20).read_to_string(&mut s); }
            Err(_) => continue,
        }
        for t in tags {
            if let Some(v) = xml_tag(&s, t) {
                if !v.trim().is_empty() { o.kv(t.split(':').last().unwrap_or(t), v); }
            }
        }
    }
}

pub fn xml_tag(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}");
    let i = xml.find(&open)?;
    let rest = &xml[i + open.len()..];
    let gt = rest.find('>')?;
    if rest[..gt].ends_with('/') { return None; }
    let body = &rest[gt + 1..];
    let end = body.find(&format!("</{tag}>"))?;
    Some(body[..end].replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn hashes_and_detects() {
        let dir = std::env::temp_dir().join(format!("psychosec-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("a.txt");
        std::fs::File::create(&p).unwrap().write_all(b"abc").unwrap();
        let o = inspect(&p).unwrap();
        let t = o.plain_lines().join("\n");
        assert!(t.contains("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"));
        let png = dir.join("fake.jpg");
        std::fs::File::create(&png).unwrap().write_all(b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR\0\0\0\x01\0\0\0\x01\x08\x02\0\0\0").unwrap();
        let t = inspect(&png).unwrap().plain_lines().join("\n");
        assert!(t.contains("does not match"));
        assert!(inspect(&dir.join("missing")).is_err());
        assert!(inspect(&dir).is_err());
        std::fs::remove_dir_all(dir).ok();
    }
    #[test]
    fn xml_and_pdf_strings() {
        assert_eq!(xml_tag("<a><dc:creator>Al &amp; Bo</dc:creator></a>", "dc:creator").unwrap(), "Al & Bo");
        assert_eq!(decode_pdf_string(&[0xFE, 0xFF, 0, 0x41, 0, 0x42]), "AB");
    }
}
