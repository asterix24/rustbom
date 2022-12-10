use anyhow::{bail, Result};
use calamine::{open_workbook_auto, DataType, Reader};
use log::{debug, error, info, warn};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::{cmp::Ordering, collections::HashMap, ffi::OsStr, fmt, path::Path, vec};
use strum_macros::EnumIter;

fn uppercase_first_letter(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

fn is_header_key(item: &str) -> Result<String> {
    let re_note = Regex::new(r"(note|code)\s(.*)").unwrap();

    match item.to_lowercase().as_str() {
        "designator" | "comment" | "footprint" | "description" | "layer" => {
            //println!("Standard: {}", item);
            Ok(uppercase_first_letter(item))
        }
        "mounttechnology" | "mount_technology" => {
            //println!("Standard: {}", item);
            Ok("MountTechnology".to_string())
        }
        _ => {
            let res: String;
            match re_note.captures(item.to_lowercase().as_ref()) {
                Some(cc) => match cc.get(0) {
                    Some(s) => {
                        res = s.as_str().to_string().to_uppercase();
                    }
                    _ => bail!("Invalid header key: {}", item),
                },
                _ => bail!("Invalid header key: {}", item),
            }
            Ok(res)
        }
    }
}

fn xlsx_loader<P: AsRef<Path>>(path: P) -> (Vec<Vec<String>>, HeaderMap) {
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
                        headers.insert(column, m);
                    } else {
                        element.push(s);
                    }
                }
                if !element.is_empty() {
                    rows.push(element.clone());
                }
            }
        }
        _ => panic!("Male.."),
    }
    (rows, headers)
}

fn csv_loader<P: AsRef<Path>>(path: P) -> (Vec<Vec<String>>, HeaderMap) {
    let mut rd = match csv::ReaderBuilder::new().has_headers(false).from_path(path) {
        Ok(r) => r,
        Err(e) => panic!("No file found {}", e),
    };

    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut headers: HeaderMap = HashMap::new();

    for line in rd.records().flatten() {
        let mut element = Vec::new();
        for (i, s) in line.iter().enumerate() {
            if let Ok(m) = is_header_key(s) {
                headers.insert(i, m);
            } else {
                element.push(s.to_string());
            }
        }
        if !element.is_empty() {
            rows.push(element.clone());
        }
    }
    (rows, headers)
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ItemView {
    pub unique_id: String,
    pub is_merged: bool,
    pub is_np: bool,
    pub category: String,
    pub fields: Vec<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ItemsTable {
    pub headers: Vec<String>,
    pub rows: Vec<ItemView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Bom {
    items: Vec<Item>,
}

impl Bom {
    pub fn loader<P: AsRef<Path>>(path: &[P]) -> Bom {
        let mut it1: Vec<Item> = Vec::new();
        if let Ok(i) = Bom::from_csv(path) {
            it1 = i;
        }

        let mut it2: Vec<Item> = Vec::new();
        if let Ok(i) = Bom::from_xlsx(path) {
            it2 = i;
        }

        it1.extend(it2);
        Bom { items: it1 }
    }

    pub fn from_csv<P: AsRef<Path>>(path: &[P]) -> Result<Vec<Item>> {
        let mut items: Vec<_> = Vec::new();

        for i in path.iter() {
            let ext = Path::new(i.as_ref())
                .extension()
                .and_then(OsStr::to_str)
                .unwrap();
            if ext != "csv" {
                warn!("{:?} {:?} != csv: skip..", i.as_ref(), ext);
                continue;
            }
            let (rows, headers) = csv_loader(i.as_ref());
            if let Ok(mut ii) = Bom::from_rows_and_headers(&rows, &headers) {
                items.append(&mut ii);
            }
        }
        Ok(items)
    }

    pub fn from_xlsx<P: AsRef<Path>>(path: &[P]) -> Result<Vec<Item>> {
        let mut items: Vec<_> = Vec::new();

        for i in path.iter() {
            let ext = Path::new(i.as_ref())
                .extension()
                .and_then(OsStr::to_str)
                .unwrap();
            if ext != "xlsx" && ext != "xls" {
                warn!("{:?} {:?} != xlsx xls: skip..", i.as_ref(), ext);
                continue;
            }
            let (rows, headers) = xlsx_loader(i);
            if let Ok(mut ii) = Bom::from_rows_and_headers(&rows, &headers) {
                items.append(&mut ii);
            }
        }
        Ok(items)
    }

    fn from_rows_and_headers(rows: &[Vec<String>], headers: &HeaderMap) -> Result<Vec<Item>> {
        let mut items: Vec<_> = Vec::new();
        for row in rows.iter() {
            if let Ok(item) = Self::parse_row(row, headers) {
                items.push(item);
            }
        }
        Ok(items)
    }

    fn parse_row(row: &[String], headers: &HeaderMap) -> Result<Item> {
        let mut items = Item::default();
        for (i, field) in row.iter().enumerate() {
            if let Some(h) = headers.get(&i) {
                if let Ok((hdr, value)) = Field::from_header_and_value(h, field) {
                    items.fields.entry(hdr.clone()).or_insert(value);
                }
            } else {
                warn!("Parse: No header {} for {}, skip it", i, field);
            };
        }
        Ok(items.guess_category().generate_uuid())
    }

    pub fn merge(&self) -> Bom {
        /* Merge policy:
        All Item element was merged by unique_id, if two row have same unique_id we could merge it in one, but:
        - if NP, skip it
        - designators is put all togheter in vector
        - quantity is increased
        */
        let mut merged: HashMap<String, Item> = HashMap::new();
        for item in self.items.iter() {
            info!("unique_id: {:?}", item.unique_id);
            if let Some(prev) = merged.get_mut(&item.unique_id) {
                /* The rows with same unique id should be merge, so first we
                get out the Fields that was mergeable */
                if let Some(Field::List(dd)) = prev.fields.get_mut("designator") {
                    if let Some(Field::List(last_dd)) = item.fields.get("designator") {
                        dd.extend(last_dd.clone());
                    }
                    dd.sort();
                    dd.dedup();
                    prev.quantity = dd.len();
                }
            } else {
                merged.insert(item.unique_id.clone(), item.clone());
            }
        }
        Bom {
            items: merged.values().cloned().collect(),
        }
    }

    pub fn odered_vector_table(&mut self) -> ItemsTable {
        let mut headers: HashMap<String, usize> = HashMap::new();
        headers.insert("quantity".to_string(), 0);
        headers.insert("designator".to_string(), 1);
        headers.insert("comment".to_string(), 2);
        headers.insert("footprint".to_string(), 3);
        headers.insert("description".to_string(), 4);
        headers.insert("layer".to_string(), 5);
        headers.insert("mounttechnology".to_string(), 6);

        // Get header map and row max len
        let mut row_capacity: usize = headers.len() - 1;
        for item in self.items.iter() {
            for hdr in item.fields.iter() {
                if !headers.contains_key(hdr.0) {
                    headers.insert(hdr.0.clone(), row_capacity);
                    row_capacity += 1;
                }
                warn!("{} {} {}", row_capacity, headers[hdr.0], hdr.0);
            }
        }

        let mut items_table = ItemsTable::default();
        self.items.sort_by(|a, b| b.category.cmp(&a.category));
        let mut header_str = Vec::from_iter(headers.iter());
        header_str.sort_by(|a, b| a.1.cmp(b.1));
        items_table.headers = header_str
            .iter()
            .map(|k| uppercase_first_letter(k.0))
            .collect();

        for item in self.items.iter() {
            let mut m: Vec<String> = vec!["".to_string(); row_capacity];
            m.insert(0, format!("{}", item.quantity));

            for k in item.fields.iter() {
                if headers.contains_key(k.0) {
                    let value = format!("{:}", k.1);
                    m[headers[k.0]] = value.clone();
                    info!("{} {} {}", row_capacity, headers[k.0], value);
                }
            }

            items_table.rows.push(ItemView {
                unique_id: item.unique_id.clone(),
                category: format!("{}", item.category),
                is_merged: item.is_merged,
                is_np: item.is_np,
                fields: m.clone(),
            });
            debug!("{:?}", m);
        }

        items_table
    }
}

pub type HeaderMap = HashMap<usize, String>;

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct Item {
    quantity: usize,
    unique_id: String,
    is_merged: bool,
    is_np: bool,
    pub category: Category,
    fields: HashMap<String, Field>,
}

impl Item {
    pub fn default() -> Self {
        let fields = HashMap::new();
        Self {
            quantity: 0,
            unique_id: "".to_string(),
            is_merged: false,
            is_np: false,
            category: Category::Invalid,
            fields,
        }
    }

    pub fn guess_category(&mut self) -> Self {
        self.category = match self.fields.get("designator") {
            Some(d) => {
                match Regex::new(r"^([a-zA-Z_]{1,3})")
                    .unwrap()
                    .captures(d.get_element_list().as_str())
                {
                    None => Category::Invalid,
                    Some(cc) => match String::from(cc.get(1).map_or("", |m| m.as_str()))
                        .to_uppercase()
                        .as_ref()
                    {
                        "J" | "X" | "P" | "SIM" => Category::Connectors,
                        "S" | "SCR" | "SPA" | "BAT" | "BUZ" | "BT" | "B" | "SW" | "MP" | "K" => {
                            Category::Mechanicals
                        }
                        "F" | "FU" => Category::Fuses,
                        "R" | "RN" | "R_G" => Category::Resistors,
                        "C" | "CAP" => Category::Capacitors,
                        "D" | "DZ" => Category::Diode,
                        "L" => Category::Inductors,
                        "Q" => Category::Transistor,
                        "TR" => Category::Transformers,
                        "Y" => Category::Cristal,
                        "U" => Category::IC,
                        _ => Category::Invalid,
                    },
                }
            }
            _ => Category::Invalid,
        };
        self.clone()
    }

    fn generate_uuid(&mut self) -> Self {
        let mut description = Field::Invalid("-".to_string());
        if let Some(d) = self.fields.get("description") {
            description = d.clone();
        };

        let mut comment = Field::Invalid("-".to_string());
        if let Some(d) = self.fields.get("comment") {
            comment = d.clone();
        };

        let mut footprint = Field::Invalid("-".to_string());
        if let Some(d) = self.fields.get("footprint") {
            footprint = d.clone();
        };

        let mut designator = Field::Invalid("-".to_string());
        if let Some(d) = self.fields.get("designator") {
            designator = d.clone();
        };

        self.is_merged = false;
        self.is_np = false;
        self.quantity = designator.get_list_len();

        self.unique_id = format!(
            "{:?}-{}-{}-{}",
            self.category, comment, footprint, description
        );

        /*
         * To merge items, we need to have a unique id computed to taking into account
         * the component category and some keywords contained in the comment field.
         * In order to avoid wrong merge, first we skip all line that are marked with
         * NP (Not-Poupulated)
         */
        match self.category {
            Category::Connectors => {
                /*
                 * In order to compute connector uid, we consider only footprint anche description
                 * because, the comment could be diffent also for same component. This also because
                 * the comment hold the label of pcb connector
                 */
                if Regex::new("^NP ")
                    .unwrap()
                    .is_match(comment.to_string().as_str())
                {
                    comment = Field::Item("NP Connector".to_string());
                } else {
                    comment = Field::Item("Connector".to_string());
                }
                self.unique_id = format!(
                    "{:?}-{}-{}-{}-connector",
                    self.category, comment, footprint, description
                );
                self.is_merged = true;
            }
            Category::Mechanicals => {
                /*
                 * Tactile switch could be have different label, but the component was same
                 */
                if footprint.to_string().to_lowercase().contains("tactile") {
                    comment = Field::Item("Tactile Switch".to_string());
                    self.unique_id =
                        format!("{:?}-{}-{}-tactile", self.category, footprint, description);
                    self.is_merged = true;
                }
            }
            Category::Diode => {
                /*
                 * Diode led could be have different label, but the component was same
                 */
                if footprint.to_string().to_lowercase().contains("LED") {
                    comment = Field::Item("Diode LED".to_string());
                    self.unique_id =
                        format!("{:?}-{}-{}-LED", self.category, footprint, description);
                    self.is_merged = true;
                }
            }
            Category::IC => {
                if footprint.to_string().to_lowercase().contains("rele")
                    || footprint.to_string().to_lowercase().contains("relay")
                {
                    comment = Field::Item("Diode LED".to_string());
                    self.unique_id =
                        format!("{:?}-{}-{}-relay", self.category, footprint, description);
                    self.is_merged = true;
                }
            }
            _ => {}
        };

        info!(
            "unique id >> {:} {:} {:} {:}",
            self.unique_id, comment, footprint, description,
        );
        self.fields.entry("comment".to_string()).or_insert(comment);
        self.clone()
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Hash, EnumIter)]
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

impl Category {
    fn enum_to_usize(&self) -> usize {
        match self {
            Self::Connectors => 11,
            Self::Mechanicals => 10,
            Self::Fuses => 9,
            Self::Resistors => 8,
            Self::Capacitors => 7,
            Self::Diode => 6,
            Self::Inductors => 5,
            Self::Transistor => 4,
            Self::Transformers => 3,
            Self::Cristal => 2,
            Self::IC => 1,
            Self::Invalid => 0,
        }
    }
}

impl Display for Category {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Connectors => write!(f, "** J Connectors **"),
            Self::Mechanicals => write!(f, "** S Mechanicals **"),
            Self::Fuses => write!(f, "** F Fuses "),
            Self::Resistors => write!(f, "** R Resistors **"),
            Self::Capacitors => write!(f, "** C Capacitors **"),
            Self::Diode => write!(f, "** D Diode **"),
            Self::Inductors => write!(f, "** L Inductors **"),
            Self::Transistor => write!(f, "** Q Transistor **"),
            Self::Transformers => write!(f, "** Tr Trasnformers **"),
            Self::Cristal => write!(f, "** Y Cristal **"),
            Self::IC => write!(f, "** U IC **"),
            Self::Invalid => write!(f, "** - Invalid **"),
        }
    }
}

impl Ord for Category {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.enum_to_usize()).cmp(&other.enum_to_usize())
    }
}

impl PartialOrd for Category {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum Field {
    List(Vec<String>),
    Item(String),
    Invalid(String),
}

impl Display for Field {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::List(v) => write!(f, "{}", v.join(", ")),
            Self::Item(s) => write!(f, "{}", s),
            Self::Invalid(s) => write!(f, "{}", s),
        }
    }
}

impl Field {
    fn get_element_list(&self) -> String {
        match self {
            Field::List(m) => {
                if let Some(x) = m.get(0) {
                    x.to_lowercase()
                } else {
                    "".to_string()
                }
            }
            _ => "".to_string(),
        }
    }

    fn get_list_len(&self) -> usize {
        match self {
            Field::List(m) => m.len(),
            _ => 0,
        }
    }

    fn get_element_item(&self) -> &str {
        match self {
            Field::Item(m) => m.as_str(),
            _ => "",
        }
    }

    fn from_header_and_value(header: &str, value: &str) -> Result<(String, Field)> {
        let mut hdr = header.to_lowercase();
        let field: Field = match hdr.as_str() {
            "designator" => Field::List(
                value
                    .to_string()
                    .split(',')
                    .map(|m| m.trim().to_string())
                    .collect(),
            ),
            "comment" | "footprint" | "description" | "mounttechnology" | "layer" => {
                Field::Item(value.to_string())
            }
            other => match Regex::new(r"(code|note)\s(.*)").unwrap().captures(other) {
                Some(cc) => match cc.get(0) {
                    Some(s) => {
                        //debug!("{:?} > h{:?} -> {:?}", s, s.as_str().to_string(), value);
                        hdr = s.as_str().to_string();
                        Field::List(vec![value.to_string().to_uppercase()])
                    }
                    _ => Field::Invalid(value.to_string()),
                },
                _ => Field::Invalid(value.to_string()),
            },
        };

        if field == Field::Invalid(value.to_string()) {
            bail!("Invalid field {} -> {}", hdr, value);
        }
        Ok((hdr, field))
    }
}
