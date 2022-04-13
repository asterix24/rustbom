use super::item::{Header, Item};

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BomFieldMap {
    header: Header,
    label: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BomRow {
    row: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Bom {
    headers: HashMap<usize, BomFieldMap>,
    rows: Vec<BomRow>,
    items: Vec<Item>,
}

impl Bom {
    pub fn new() -> Bom {
        Self {
            headers: HashMap::new(),
            rows: vec![],
            items: vec![],
        }
    }
    pub fn insert_if_header(&mut self, index: usize, item: &str) {
        let mut header_map = BomFieldMap {
            header: Header::Invalid,
            label: "".to_string(),
        };
        let header_field = is_header_key(item);
        if header_field.header != Header::Invalid {
            header_map.header = header_field.header;
            header_map.label = header_field.label;
            self.headers.insert(index, header_map);
        }
    }
    pub fn insert_row(&mut self, row: &[String]) {
        if !row.is_empty() {
            self.rows.push(BomRow {
                row: row.to_owned(),
            });
        }
    }
}

fn is_header_key(item: &str) -> BomFieldMap {
    let re_note = Regex::new(r"note\s(.*)").unwrap();
    let re_code = Regex::new(r"code\s(.*)").unwrap();

    match item.to_lowercase().as_str() {
        "designator" => BomFieldMap {
            header: Header::Designator,
            label: "".to_string(),
        },
        "comment" => BomFieldMap {
            header: Header::Comment,
            label: "".to_string(),
        },
        "footprint" => BomFieldMap {
            header: Header::Footprint,
            label: "".to_string(),
        },
        "description" => BomFieldMap {
            header: Header::Description,
            label: "".to_string(),
        },
        "mounttechnology" | "mount_technology" => BomFieldMap {
            header: Header::MountTecnology,
            label: "".to_string(),
        },
        "layer" => BomFieldMap {
            header: Header::Layer,
            label: "".to_string(),
        },
        _ => {
            let mut value = Header::Invalid;
            let mut label = "".to_string();
            if let Some(cc) = re_code.captures(item.to_lowercase().as_ref()) {
                if let Some(m) = cc.get(1).map(|m| m.as_str()) {
                    value = Header::ExtraCode;
                    label = m.to_string();
                }
            }
            if let Some(cc) = re_note.captures(item.to_lowercase().as_ref()) {
                if let Some(m) = cc.get(1).map(|m| m.as_str()) {
                    value = Header::ExtraNote;
                    label = m.to_string();
                }
            }

            BomFieldMap {
                header: value,
                label,
            }
        }
    }
}
