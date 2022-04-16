use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use anyhow::{bail, Result};
use calamine::{open_workbook_auto, DataType, Reader};
use serde::{Deserialize, Serialize};

use regex::Regex;

fn uppercase_first_letter(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

fn is_header_key(item: &str) -> Result<String> {
    let re_note = Regex::new(r"note\s(.*)").unwrap();
    let re_code = Regex::new(r"code\s(.*)").unwrap();

    match item.to_lowercase().as_str() {
        "designator" | "comment" | "footprint" | "description" | "layer" => {
            println!("{}", item);
            Ok(uppercase_first_letter(item))
        }
        "mounttechnology" | "mount_technology" => Ok("Mount Technology".to_string()),
        _ => {
            let mut res = "".to_string();
            if let Some(cc) = re_code.captures(item.to_lowercase().as_ref()) {
                if let Some(m) = cc.get(1).map(|m| m.as_str()) {
                    res = format!("Code {}", uppercase_first_letter(m));
                }
            }
            if let Some(cc) = re_note.captures(item.to_lowercase().as_ref()) {
                if let Some(m) = cc.get(1).map(|m| m.as_str()) {
                    res = format!("Note {}", uppercase_first_letter(m));
                }
            }
            if res.is_empty() {
                bail!("Invalid header key")
            } else {
                println!("{}", res);
                Ok(res)
            }
        }
    }
}

fn generate_uuid(item: Item) -> Result<Item> {
    /*
     * To merge items, we need to have a unique id comuted taking into account
     * the component category and some keywords contained in the comment field.
     * In order to avoid wrong merge, first we skip all line that are marked with
     * NP (Not-Poupulated)
     */

    let mut description = "".to_string();
    let mut comment = "".to_string();
    let mut footprint = "".to_string();
    let mut unique_id = "".to_string();
    let mut fields = HashSet::new();

    for i in item.fields.iter() {
        match i {
            Field::Description(d) => description = d.clone(),
            Field::Comment(c) => comment = c.clone(),
            Field::Footprint(f) => footprint = f.clone(),
            _ => (),
        }
    }

    for v in item.fields.iter() {
        match v {
            Field::Comment(s) => {
                if Regex::new("^NP ").unwrap().is_match(s) {
                    fields.insert(Field::IsNP(true));
                }
            }
            Field::Category(c) => match c {
                Category::Connectors => {
                    if Regex::new("^NP ").unwrap().is_match(comment.as_str()) {
                        comment = "NP Connector".to_string();
                    } else {
                        comment = "Connector".to_string();
                    }
                    unique_id = format!("{}-{}-{}-connector", description, comment, footprint);

                    // Overwrite comment, we want merge connectors, that they
                    // could have different comment, so we need to replace it
                    fields.insert(Field::Comment(comment.clone()));
                    fields.insert(Field::UniqueId(unique_id.clone()));
                    fields.insert(Field::IsMerged(true));
                }
                Category::Mechanicals => {
                    if footprint.to_lowercase().contains("tactile") {
                        unique_id = format!("{}-{}-tactile", description, footprint);

                        // Overwrite comment, we want merge connectors, that they
                        // could have different comment, so we need to replace it
                        fields.insert(Field::Comment("Tactile Switch".to_string()));
                        fields.insert(Field::UniqueId(unique_id.clone()));
                        fields.insert(Field::IsMerged(true));
                    }
                }
                Category::Diode => {
                    if footprint.to_lowercase().contains("LED") {
                        unique_id = format!("{}-{}-LED", description, footprint);

                        // Overwrite comment, we want merge connectors, that they
                        // could have different comment, so we need to replace it
                        fields.insert(Field::Comment("Diode LED".to_string()));
                        fields.insert(Field::UniqueId(unique_id.clone()));
                        fields.insert(Field::IsMerged(true));
                    }
                }
                Category::IC => {
                    if footprint.to_lowercase().contains("rele")
                        || footprint.to_lowercase().contains("relay")
                    {
                        unique_id = format!("{}-{}-relay", description, footprint);

                        // Overwrite comment, we want merge connectors, that they
                        // could have different comment, so we need to replace it
                        fields.insert(Field::Comment("Relay, Rele\'".to_string()));
                        fields.insert(Field::UniqueId(unique_id.clone()));
                        fields.insert(Field::IsMerged(true));
                    }
                }
                _ => {
                    fields.insert(Field::UniqueId(format!(
                        "{}-{}-{}",
                        description, comment, footprint
                    )));
                    fields.insert(Field::IsNP(false));
                    fields.insert(Field::IsMerged(false));
                }
            },
            Field::ExtraCode(c) | Field::ExtraNote(c) => {
                fields.insert(Field::UniqueId(format!(
                    "{}-extra-{}",
                    unique_id.clone(),
                    c.join("-")
                )));
            }
            other => {
                println!("{:?}", other);
                fields.insert(other.clone());
            }
        }
    }
    Ok(Item { fields })
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Bom {
    items: Vec<Item>,
}

impl Bom {
    pub fn from_csv<P: AsRef<Path>>(path: P) -> Result<Bom> {
        let mut rd = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_path(path)?;

        let headers: HeaderMap = rd.headers()?.iter().map(String::from).enumerate().collect();

        let rows = rd
            .into_records()
            .flatten()
            .map(|record| record.iter().map(String::from).collect::<Vec<_>>())
            .collect::<Vec<_>>();

        Bom::from_rows_and_headers(&rows, &headers)
    }

    pub fn from_xlsx<P: AsRef<Path>>(path: P) -> Result<Bom> {
        let mut workbook = match open_workbook_auto(path) {
            Ok(wb) => wb,
            Err(e) => panic!("{} Error while parsing file", e),
        };
        let sheet_name = match workbook.sheet_names().first() {
            Some(name) => name.to_string(),
            None => panic!("No sheet found in file"),
        };

        let mut headers: HeaderMap = HashMap::new();
        let mut rows: Vec<Vec<String>> = Vec::new();
        match workbook.worksheet_range(sheet_name.as_str()) {
            Some(Ok(range)) => {
                let (rw, cl) = range.get_size();
                for row in 0..rw {
                    let mut element = Vec::new();
                    for column in 0..cl {
                        let s = match range.get((row, column)) {
                            Some(DataType::String(s)) => s.to_string(),
                            Some(DataType::Int(s)) => s.to_string(),
                            Some(DataType::Float(s)) => s.to_string(),
                            _ => "-".to_string(),
                        };
                        if let Ok(m) = is_header_key(&s) {
                            println!(">> {} {}", m, s);
                            headers.insert(column, m);
                        }
                        element.push(s);
                    }
                    if !element.is_empty() {
                        rows.push(element.clone());
                    }
                }
            }
            _ => panic!("Male.."),
        }

        println!("... {:?}", headers);
        Bom::from_rows_and_headers(&rows, &headers)
    }

    fn from_rows_and_headers(rows: &[Vec<String>], headers: &HeaderMap) -> Result<Bom> {
        let mut items: Vec<_> = Vec::new();
        for row in rows.iter() {
            match Self::parse_row(row, headers) {
                Ok(item) => items.push(item),
                Err(e) => println!("{}", e),
            }
        }

        Ok(Bom { items })
    }

    fn parse_row(row: &[String], headers: &HeaderMap) -> Result<Item> {
        let mut fields = HashSet::new();

        for (i, field) in row.iter().enumerate() {
            let header = match headers.get(&i) {
                Some(h) => h,
                None => {
                    println!("No header for {}, skip it", i);
                    ""
                }
            };

            if header.is_empty() {
                continue;
            }

            if let Ok(m) = Field::from_header_and_value(header, field) {
                fields.insert(m);
            } else {
                println!("Not Valid {} {}", header, field);
            }

            if let Ok(m) = Field::category_from_designator(header, field) {
                fields.insert(m);
            } else {
                println!("Not Valid {} {}", header, field);
            }
        }

        let fields_with_uuid = generate_uuid(Item { fields })?;

        Ok(fields_with_uuid)
    }

    pub fn items(&self) -> &[Item] {
        &self.items
    }
}

fn guess_category<S: AsRef<str>>(designator: S) -> Result<Category> {
    match Regex::new(r"^([a-zA-Z_]{1,3})")
        .unwrap()
        .captures(designator.as_ref())
    {
        None => Ok(Category::Invalid),
        Some(cc) => match String::from(cc.get(1).map_or("", |m| m.as_str()))
            .to_uppercase()
            .as_ref()
        {
            "J" | "X" | "P" | "SIM" => Ok(Category::Connectors),
            "S" | "SCR" | "SPA" | "BAT" | "BUZ" | "BT" | "B" | "SW" | "MP" | "K" => {
                Ok(Category::Mechanicals)
            }
            "F" | "FU" => Ok(Category::Fuses),
            "R" | "RN" | "R_G" => Ok(Category::Resistors),
            "C" | "CAP" => Ok(Category::Capacitors),
            "D" | "DZ" => Ok(Category::Diode),
            "L" => Ok(Category::Inductors),
            "Q" => Ok(Category::Transistor),
            "TR" => Ok(Category::Transformers),
            "Y" => Ok(Category::Cristal),
            "U" => Ok(Category::IC),
            _ => bail!("Invalid category[{:#?}]", designator.as_ref()),
        },
    }
}

pub type HeaderMap = HashMap<usize, String>;

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct Item {
    fields: HashSet<Field>,
}
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum Category {
    Connectors,
    Mechanicals,
    Fuses,
    Resistors,
    Capacitors,
    Diode,
    Inductors,
    Transistor,
    Transformers,
    Cristal,
    IC,
    Invalid,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum Field {
    UniqueId(String),
    IsMerged(bool),
    IsNP(bool),
    Category(Category),
    Designator(Vec<String>),
    Comment(String),
    Footprint(String),
    Description(String),
    ExtraCode(Vec<String>),
    ExtraNote(Vec<String>),
    Invalid(String),
}

impl Field {
    fn from_header_and_value(header: &str, value: &str) -> Result<Field> {
        let field: Field = match header {
            "Designator" => {
                Field::Designator(value.to_string().split(',').map(String::from).collect())
            }
            "Comment" => Field::Comment(value.to_string()),
            "Footprint" => Field::Footprint(value.to_string()),
            "Description" => Field::Description(value.to_string()),
            _ => Field::Invalid("-".to_string()),
        };

        if field == Field::Invalid("-".to_string()) {
            bail!("Invalid field {}", header);
        } else {
            Ok(field)
        }
    }
    fn category_from_designator(header: &str, value: &str) -> Result<Field> {
        if header == "Designator" {
            let category = guess_category(value)?;
            Ok(Field::Category(category))
        } else {
            bail!("No valid category field {}", value);
        }
    }
}
