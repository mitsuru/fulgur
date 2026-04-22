use serde::Serialize;
use std::path::Path;

#[derive(Debug, Serialize, PartialEq)]
pub struct InspectResult {
    pub pages: u32,
    pub metadata: Metadata,
    pub text_items: Vec<TextItem>,
    pub images: Vec<ImageItem>,
}

#[derive(Debug, Serialize, PartialEq, Default)]
pub struct Metadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creator: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_at: Option<String>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct TextItem {
    pub page: u32,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub text: String,
    pub font: String,
    pub font_size: f32,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct ImageItem {
    pub page: u32,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub format: String,
    pub width_px: u32,
    pub height_px: u32,
}

pub fn inspect(path: &Path) -> crate::Result<InspectResult> {
    let doc = lopdf::Document::load(path)
        .map_err(|e| crate::Error::Other(format!("Failed to load PDF: {e}")))?;

    let pages = doc.get_pages().len() as u32;
    let metadata = extract_metadata(&doc);
    let text_items = extract_text_items(&doc)?;
    let images = extract_image_items(&doc)?;

    Ok(InspectResult {
        pages,
        metadata,
        text_items,
        images,
    })
}

fn obj_as_name_str(obj: &lopdf::Object) -> Option<&str> {
    obj.as_name().ok().and_then(|b| std::str::from_utf8(b).ok())
}

fn extract_metadata(doc: &lopdf::Document) -> Metadata {
    let mut meta = Metadata::default();
    let info_id = match doc.trailer.get(b"Info") {
        Ok(obj) => match obj.as_reference() {
            Ok(id) => id,
            Err(_) => return meta,
        },
        Err(_) => return meta,
    };
    let info = match doc.get_object(info_id) {
        Ok(lopdf::Object::Dictionary(d)) => d,
        _ => return meta,
    };

    let get_str = |dict: &lopdf::Dictionary, key: &[u8]| -> Option<String> {
        dict.get(key)
            .ok()
            .and_then(|o| o.as_str().ok())
            .map(|b| String::from_utf8_lossy(b).into_owned())
    };

    meta.title = get_str(info, b"Title");
    meta.author = get_str(info, b"Author");
    meta.creator = get_str(info, b"Creator");
    meta.created_at = get_str(info, b"CreationDate");
    meta.modified_at = get_str(info, b"ModDate");
    meta
}

fn extract_text_items(doc: &lopdf::Document) -> crate::Result<Vec<TextItem>> {
    use lopdf::content::Operation;
    let mut items = Vec::new();

    for (&page_num, &page_id) in &doc.get_pages() {
        let content_bytes = match doc.get_page_content(page_id) {
            Ok(b) => b,
            Err(_) => continue,
        };
        let content = match lopdf::content::Content::decode(&content_bytes) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let mut tx: f32 = 0.0;
        let mut ty: f32 = 0.0;
        let mut font_name = String::from("unknown");
        let mut font_size: f32 = 12.0;

        for Operation { operator, operands } in &content.operations {
            match operator.as_str() {
                "Tf" => {
                    if let (Some(name_obj), Some(size)) = (operands.first(), operands.get(1)) {
                        font_name = obj_as_name_str(name_obj)
                            .unwrap_or("unknown")
                            .to_string();
                        font_size = obj_to_f32(size);
                    }
                }
                "Tm" => {
                    if operands.len() >= 6 {
                        tx = obj_to_f32(&operands[4]);
                        ty = obj_to_f32(&operands[5]);
                    }
                }
                "Td" | "TD" => {
                    if operands.len() >= 2 {
                        tx += obj_to_f32(&operands[0]);
                        ty += obj_to_f32(&operands[1]);
                    }
                }
                "T*" => {
                    ty -= font_size;
                }
                "Tj" => {
                    if let Some(text_obj) = operands.first() {
                        if let Ok(bytes) = text_obj.as_str() {
                            let text = decode_pdf_string(bytes);
                            if !text.trim().is_empty() {
                                let w = estimate_width(&text, font_size);
                                items.push(TextItem {
                                    page: page_num,
                                    x: tx,
                                    y: ty,
                                    width: w,
                                    height: font_size,
                                    text,
                                    font: font_name.clone(),
                                    font_size,
                                });
                                tx += w;
                            }
                        }
                    }
                }
                "TJ" => {
                    if let Some(array_obj) = operands.first() {
                        if let Ok(array) = array_obj.as_array() {
                            let mut combined = String::new();
                            for elem in array {
                                if let Ok(bytes) = elem.as_str() {
                                    combined.push_str(&decode_pdf_string(bytes));
                                }
                            }
                            if !combined.trim().is_empty() {
                                let w = estimate_width(&combined, font_size);
                                items.push(TextItem {
                                    page: page_num,
                                    x: tx,
                                    y: ty,
                                    width: w,
                                    height: font_size,
                                    text: combined,
                                    font: font_name.clone(),
                                    font_size,
                                });
                                tx += w;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
    Ok(items)
}

fn extract_image_items(doc: &lopdf::Document) -> crate::Result<Vec<ImageItem>> {
    let mut items = Vec::new();
    let pages = doc.get_pages();

    for (&page_num, &page_id) in &pages {
        let page_obj = match doc.get_object(page_id) {
            Ok(lopdf::Object::Dictionary(d)) => d.clone(),
            _ => continue,
        };

        let resources = match page_obj.get(b"Resources") {
            Ok(res) => {
                let resolved = doc.dereference(res).map(|(_, o)| o);
                match resolved {
                    Ok(lopdf::Object::Dictionary(d)) => d.clone(),
                    _ => continue,
                }
            }
            Err(_) => continue,
        };

        let xobjects = match resources.get(b"XObject") {
            Ok(xo) => {
                let resolved = doc.dereference(xo).map(|(_, o)| o);
                match resolved {
                    Ok(lopdf::Object::Dictionary(d)) => d.clone(),
                    _ => continue,
                }
            }
            Err(_) => continue,
        };

        let mut image_names: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        for (name, obj_ref) in xobjects.iter() {
            let xobj = match doc.dereference(obj_ref).map(|(_, o)| o) {
                Ok(lopdf::Object::Stream(s)) => s,
                _ => continue,
            };
            let subtype = xobj
                .dict
                .get(b"Subtype")
                .ok()
                .and_then(|o| obj_as_name_str(o))
                .unwrap_or("");
            if subtype == "Image" {
                let fmt = detect_image_format(&xobj.dict);
                let w_px = xobj
                    .dict
                    .get(b"Width")
                    .ok()
                    .and_then(|o| o.as_i64().ok())
                    .unwrap_or(0) as u32;
                let h_px = xobj
                    .dict
                    .get(b"Height")
                    .ok()
                    .and_then(|o| o.as_i64().ok())
                    .unwrap_or(0) as u32;
                let name_str = String::from_utf8_lossy(name).into_owned();
                image_names.insert(name_str.clone());
                items.push(ImageItem {
                    page: page_num,
                    x: 0.0,
                    y: 0.0,
                    width: 0.0,
                    height: 0.0,
                    format: fmt,
                    width_px: w_px,
                    height_px: h_px,
                });
            }
        }

        if image_names.is_empty() {
            continue;
        }

        let content_bytes = match doc.get_page_content(page_id) {
            Ok(b) => b,
            Err(_) => continue,
        };
        let content = match lopdf::content::Content::decode(&content_bytes) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let mut ctm = [1.0f32, 0.0, 0.0, 1.0, 0.0, 0.0];
        for op in &content.operations {
            match op.operator.as_str() {
                "cm" if op.operands.len() == 6 => {
                    ctm = [
                        obj_to_f32(&op.operands[0]),
                        obj_to_f32(&op.operands[1]),
                        obj_to_f32(&op.operands[2]),
                        obj_to_f32(&op.operands[3]),
                        obj_to_f32(&op.operands[4]),
                        obj_to_f32(&op.operands[5]),
                    ];
                }
                "Do" => {
                    if let Some(name_obj) = op.operands.first() {
                        let name = obj_as_name_str(name_obj).unwrap_or("");
                        if image_names.contains(name) {
                            for item in items.iter_mut().filter(|i| i.page == page_num) {
                                if item.x == 0.0
                                    && item.y == 0.0
                                    && item.width == 0.0
                                {
                                    item.x = ctm[4];
                                    item.y = ctm[5];
                                    item.width = ctm[0].abs();
                                    item.height = ctm[3].abs();
                                    break;
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
    Ok(items)
}

fn obj_to_f32(obj: &lopdf::Object) -> f32 {
    match obj {
        lopdf::Object::Integer(i) => *i as f32,
        lopdf::Object::Real(f) => *f as f32,
        _ => 0.0,
    }
}

fn decode_pdf_string(bytes: &[u8]) -> String {
    if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
        let chars: Vec<u16> = bytes[2..]
            .chunks(2)
            .filter(|c| c.len() == 2)
            .map(|c| u16::from_be_bytes([c[0], c[1]]))
            .collect();
        return String::from_utf16_lossy(&chars);
    }
    bytes.iter().map(|&b| b as char).collect()
}

fn estimate_width(text: &str, font_size: f32) -> f32 {
    text.chars().count() as f32 * font_size * 0.5
}

fn detect_image_format(dict: &lopdf::Dictionary) -> String {
    if let Ok(filter) = dict.get(b"Filter") {
        let name = match filter {
            lopdf::Object::Name(n) => String::from_utf8_lossy(n).into_owned(),
            lopdf::Object::Array(arr) => arr
                .last()
                .and_then(|o| obj_as_name_str(o))
                .unwrap_or("")
                .to_string(),
            _ => String::new(),
        };
        match name.as_str() {
            "DCTDecode" => return "jpeg".to_string(),
            "JPXDecode" => return "jp2".to_string(),
            "CCITTFaxDecode" => return "tiff".to_string(),
            "FlateDecode" => return "png".to_string(),
            _ => {}
        }
    }
    "unknown".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_test_pdf(html: &str) -> Vec<u8> {
        crate::engine::Engine::builder()
            .build()
            .render_html(html)
            .unwrap()
    }

    fn inspect_bytes(bytes: &[u8]) -> InspectResult {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), bytes).unwrap();
        inspect(tmp.path()).unwrap()
    }

    #[test]
    fn inspect_page_count() {
        let pdf = render_test_pdf("<html><body><p>Hello</p></body></html>");
        let result = inspect_bytes(&pdf);
        assert_eq!(result.pages, 1);
    }

    #[test]
    fn inspect_metadata_title() {
        let pdf = crate::engine::Engine::builder()
            .title("Test Title".to_string())
            .build()
            .render_html("<html><body><p>Hi</p></body></html>")
            .unwrap();
        let result = inspect_bytes(&pdf);
        assert_eq!(result.metadata.title.as_deref(), Some("Test Title"));
    }

    #[test]
    fn inspect_text_items_non_empty() {
        let pdf = render_test_pdf("<html><body><p>Hello World</p></body></html>");
        let result = inspect_bytes(&pdf);
        assert!(!result.text_items.is_empty(), "expected text items");
    }

    #[test]
    fn inspect_text_item_fields() {
        let pdf = render_test_pdf("<html><body><p>Hello</p></body></html>");
        let result = inspect_bytes(&pdf);
        if let Some(item) = result.text_items.first() {
            assert!(item.page >= 1);
            assert!(item.font_size > 0.0);
            assert!(!item.text.is_empty());
        }
    }

    #[test]
    fn inspect_result_serializes_to_json() {
        let pdf = render_test_pdf("<html><body><p>Test</p></body></html>");
        let result = inspect_bytes(&pdf);
        let json = serde_json::to_string_pretty(&result).unwrap();
        assert!(json.contains("\"pages\""));
        assert!(json.contains("\"metadata\""));
        assert!(json.contains("\"text_items\""));
        assert!(json.contains("\"images\""));
    }
}
