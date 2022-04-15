// use super::bom::{BomFieldMap, BomRow};
// use super::utils::guess_category;

// use lazy_static::lazy_static;
// use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, PartialEq, PartialOrd, Clone, Eq, Ord, Deserialize, Serialize)]
pub enum Category {
    Connectors,
    Mechanicals,
    Fuses,
    Resistors,
    Capacitors,
    Diode,
    Inductors,
    Transistor,
    Transformes,
    Cristal,
    IC,
    Invalid,
}

#[derive(Debug, PartialEq, PartialOrd, Clone, Eq, Ord, Copy, Deserialize, Serialize)]
pub enum Header {
    Quantity,
    Designator,
    Comment,
    Footprint,
    Description,
    MountTecnology,
    Layer,
    ExtraCode,
    ExtraNote,
    Invalid,
}

impl fmt::Display for Category {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

// #[derive(Debug, Clone, Deserialize, Serialize)]
// pub struct ExtraCol {
//     pub label: String,
//     pub value: String,
// }

// #[derive(Debug, Clone, Deserialize, Serialize)]
// pub struct Item {
//     unique_id: String,
//     is_merged: bool,
//     is_not_populated: bool,
//     category: Category,
//     designator: Vec<String>,
//     comment: String,
//     footprint: String,
//     description: String,
//     mount_type: String,
//     layer: String,
//     extra: Vec<ExtraCol>,
// }

// impl Item {
//     pub fn new() -> Self {
//         Self {
//             unique_id: "".to_string(),
//             is_merged: false,
//             is_not_populated: false,
//             category: Category::Invalid,
//             designator: vec![],
//             comment: "".to_string(),
//             footprint: "".to_string(),
//             description: "".to_string(),
//             mount_type: "".to_string(),
//             layer: "".to_string(),
//             extra: vec![],
//         }
//     }

//     pub fn get_unique_id(&self) -> &str {
//         &self.unique_id
//     }

//     pub fn insert(&mut self, row: &BomRow, header: &BomFieldMap) {
//         for (n, field) in bom_row.row.iter().enumerate() {
//             let (hdr, label) = match bom_header.get(&n) {
//                 Some(h) => h,
//                 None => continue,
//             };
//             // traspone raw data to item data
//             match hdr {
//                 Header::Designator => {
//                     self.designator = field.split(',').map(|s| s.to_string()).collect();
//                 }
//                 Header::Comment => {
//                     self.comment = field.clone();
//                 }
//                 Header::Footprint => {
//                     self.footprint = field.clone();
//                 }
//                 Header::Description => {
//                     self.description = field.clone();
//                 }
//                 Header::MountTecnology => {
//                     self.mount_type = field.clone();
//                 }
//                 Header::Layer => {
//                     self.layer = field.clone();
//                 }
//                 Header::ExtraCode => {
//                     self.extra.push(ExtraCol {
//                         label: format!("Code {}", label).to_string(),
//                         value: field.clone(),
//                     });
//                 }
//                 Header::ExtraNote => {
//                     self.extra.push(ExtraCol {
//                         label: format!("Note {}", label).to_string(),
//                         value: field.clone(),
//                     });
//                 }
//                 _ => {
//                     // do nothing
//                 }
//             }

//             /*
//              * To merge items, we need to have a unique id comuted taking into account
//              * the component category and some keywords contained in the comment field.
//              * In order to avoid wrong merge, first we skip all line that are marked with
//              * NP (Not-Poupulated)
//              */
//             lazy_static! {
//                 static ref RE: Regex = Regex::new("^NP ").unwrap();
//             }
//             if RE.is_match(&self.comment) {
//                 self.is_not_populated = true;
//             }

//             self.category = guess_category(self.designator.first().unwrap_or(&"".to_string()));
//             match self.category {
//                 Category::Connectors => {
//                     if RE.is_match(&self.comment) {
//                         self.comment = "NP Connector".to_string();
//                         self.is_not_populated = true;
//                     } else {
//                         self.comment = "Connector".to_string();
//                     }
//                     self.unique_id =
//                         format!("{}-{}-{}", self.description, self.comment, self.footprint);
//                     self.is_merged = true;
//                 }
//                 Category::Mechanicals => {
//                     if self.footprint.to_lowercase().contains("tactile") {
//                         self.unique_id = format!("{}-{}", self.description, self.footprint);
//                         self.comment = "Tactile Switch".to_string();
//                         self.is_merged = true;
//                     }
//                 }
//                 Category::Diode => {
//                     if self.footprint.to_lowercase().contains("LED") {
//                         self.unique_id = format!("{}-{}", self.description, self.footprint);
//                         self.comment = "Diode LED".to_string();
//                         self.is_merged = true;
//                     }
//                 }
//                 Category::IC => {
//                     if self.footprint.to_lowercase().contains("rele")
//                         || self.footprint.to_lowercase().contains("relay")
//                     {
//                         self.unique_id = format!("{}-{}", self.description, self.footprint);
//                         self.comment = "Relay, Rele\'".to_string();
//                         self.is_merged = true;
//                     }
//                 }
//                 _ => {
//                     self.unique_id =
//                         format!("{}-{}-{}", self.description, self.comment, self.footprint);
//                 }
//             }
//             for i in &self.extra {
//                 self.unique_id = format!("{}-{}", self.unique_id, i.value);
//             }
//         }
//     }
// }

// use super::item::{Header, Item};

// use regex::Regex;
// use serde::{Deserialize, Serialize};
// use std::collections::HashMap;

// #[derive(Debug, Clone, Deserialize, Serialize)]
// pub struct BomFieldMap {
//     header: Header,
//     label: String,
// }
// #[derive(Debug, Clone, Deserialize, Serialize)]
// pub struct BomRow {
//     row: Vec<String>,
// }

// #[derive(Debug, Clone, Deserialize, Serialize)]
// pub struct Bom {
//     headers: HashMap<usize, BomFieldMap>,
//     rows: Vec<BomRow>,
//     items: Vec<Item>,
// }

// impl Bom {
//     pub fn new() -> Bom {
//         Self {
//             headers: HashMap::new(),
//             rows: vec![],
//             items: vec![],
//         }
//     }
//     pub fn insert_if_header(&mut self, index: usize, item: &str) {
//         let mut header_map = BomFieldMap {
//             header: Header::Invalid,
//             label: "".to_string(),
//         };
//         let header_field = is_header_key(item);
//         if header_field.header != Header::Invalid {
//             header_map.header = header_field.header;
//             header_map.label = header_field.label;
//             self.headers.insert(index, header_map);
//         }
//     }
//     pub fn insert_row(&mut self, row: &[String]) {
//         if !row.is_empty() {
//             self.rows.push(BomRow {
//                 row: row.to_owned(),
//             });
//         }
//     }
// }

// fn is_header_key(item: &str) -> BomFieldMap {
//     let re_note = Regex::new(r"note\s(.*)").unwrap();
//     let re_code = Regex::new(r"code\s(.*)").unwrap();

//     match item.to_lowercase().as_str() {
//         "designator" => BomFieldMap {
//             header: Header::Designator,
//             label: "".to_string(),
//         },
//         "comment" => BomFieldMap {
//             header: Header::Comment,
//             label: "".to_string(),
//         },
//         "footprint" => BomFieldMap {
//             header: Header::Footprint,
//             label: "".to_string(),
//         },
//         "description" => BomFieldMap {
//             header: Header::Description,
//             label: "".to_string(),
//         },
//         "mounttechnology" | "mount_technology" => BomFieldMap {
//             header: Header::MountTecnology,
//             label: "".to_string(),
//         },
//         "layer" => BomFieldMap {
//             header: Header::Layer,
//             label: "".to_string(),
//         },
//         _ => {
//             let mut value = Header::Invalid;
//             let mut label = "".to_string();
//             if let Some(cc) = re_code.captures(item.to_lowercase().as_ref()) {
//                 if let Some(m) = cc.get(1).map(|m| m.as_str()) {
//                     value = Header::ExtraCode;
//                     label = m.to_string();
//                 }
//             }
//             if let Some(cc) = re_note.captures(item.to_lowercase().as_ref()) {
//                 if let Some(m) = cc.get(1).map(|m| m.as_str()) {
//                     value = Header::ExtraNote;
//                     label = m.to_string();
//                 }
//             }

//             BomFieldMap {
//                 header: value,
//                 label,
//             }
//         }
//     }
// }
