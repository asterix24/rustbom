use super::utils::guess_category;
use serde::{Deserialize, Serialize};

use std::fmt;
use std::collections::HashMap;
use lazy_static::lazy_static;
use regex::Regex;

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
    IVALID,
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
    INVALID,
}


impl fmt::Display for Category {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExtraCol {
    pub label: String,
    pub value: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Item {
    unique_id: String,
    is_merged: bool,
    is_not_populated: bool,
    category: Category,
    designator: Vec<String>,
    comment: String,
    footprint: String,
    description: String,
    mount_type: String,
    layer: String,
    extra: Vec<ExtraCol>,
}
impl Item {
    pub fn new(
    ) -> Self {
        let local_item = Self {
            unique_id: "".to_string(),
            is_merged: false,
            is_not_populated: false,
            category: Category::IVALID,
            designator: vec![],
            comment: "".to_string(),
            footprint: "".to_string(),  
            description: "".to_string(),
            mount_type: "".to_string(),
            layer: "".to_string(),
            extra: vec![],
        };
        local_item
    }
    pub fn get_unique_id(&self) -> &str {
        &self.unique_id
    }

    pub fn push(&mut self, row: &Vec<String>, header: &HashMap<usize, (Header, String)>) {
        for (n, field) in row.iter().enumerate() {
            let (hdr, label) = match header.get(&n) {
                Some(h) => h,
                None => continue,
            };
            // traspone raw data to item data
            match hdr {
                Header::Designator => {
                    self.designator = field.split(",").map(|s| s.to_string()).collect();
                },
                Header::Comment => {
                    self.comment = field.clone();
                },
                Header::Footprint => {
                    self.footprint = field.clone();
                },
                Header::Description => {
                    self.description = field.clone();
                },
                Header::MountTecnology => {
                    self.mount_type = field.clone();
                },
                Header::Layer => {
                    self.layer = field.clone();
                },
                Header::ExtraCode => {
                    self.extra.push(ExtraCol {
                        label: format!("Code {}", label).to_string(),
                        value: field.clone(),
                    });
                }
                Header::ExtraNote => {
                    self.extra.push(ExtraCol {
                        label: format!("Note {}", label).to_string(),
                        value: field.clone(),
                    });
                }
                _ => {
                    // do nothing
                },
            }
            
            /* 
            * To merge items, we need to have a unique id comuted taking into account
            * the component category and some keywords contained in the comment field.
            * In order to avoid wrong merge, first we skip all line that are marked with
            * NP (Not-Poupulated)
            */
            lazy_static! {
                static ref RE: Regex = Regex::new("^NP ").unwrap();
            }
            if RE.is_match(&self.comment) {
                self.is_not_populated = true;
            }
            
            self.category = guess_category(self.designator.first().unwrap_or(&"".to_string()));
            match self.category {
                Category::Connectors => {
                    if RE.is_match(&self.comment) {
                        self.comment = "NP Connector".to_string();
                        self.is_not_populated = true;
                    } else {
                        self.comment = "Connector".to_string();
                    }
                    self.unique_id = format!("{}-{}-{}", self.description, self.comment, self.footprint);
                    self.is_merged = true;
                }
                Category::Mechanicals => {
                    if self.footprint.to_lowercase().contains("tactile") {
                        self.unique_id = format!("{}-{}", self.description, self.footprint);
                        self.comment = "Tactile Switch".to_string();
                        self.is_merged = true;
                    }
                },
                Category::Diode => {
                    if self.footprint.to_lowercase().contains("LED") {
                        self.unique_id = format!("{}-{}", self.description, self.footprint);
                        self.comment = "Diode LED".to_string();
                        self.is_merged = true;
                    }
                }
                Category::IC => {
                    if self.footprint.to_lowercase().contains("rele") ||
                     self.footprint.to_lowercase().contains("relay") {
                        self.unique_id = format!("{}-{}", self.description, self.footprint);
                        self.comment = "Relay, Rele\'".to_string();
                        self.is_merged = true;
                    }
                }
                _ => {
                    self.unique_id = format!("{}-{}-{}", self.description, self.comment, self.footprint);
                },
            }
            for i in &self.extra {
                self.unique_id = format!("{}-{}", self.unique_id, i.value);
            }
        }
    }
}

