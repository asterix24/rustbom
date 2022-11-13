use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
    ffi::OsStr,
    fmt,
    path::Path,
    vec,
};

use anyhow::{bail, Result};
use calamine::{open_workbook_auto, DataType, Reader};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
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
    pub quantity: usize,
    pub unique_id: String,
    pub is_merged: bool,
    pub is_np: bool,
    pub category: String,
    pub fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ItemsTable {
    pub headers: Vec<String>,
    pub rows: Vec<ItemView>,
}

impl Default for ItemsTable {
    fn default() -> Self {
        ItemsTable {
            headers: vec![
                "Quantity".to_string(),
                "Designator".to_string(),
                "Comment".to_string(),
                "Footprint".to_string(),
                "Description".to_string(),
                "Layer".to_string(),
                "MountTechnology".to_string(),
            ],
            rows: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Bom {
    items: Vec<Item>,
}

impl Ord for Field {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.enum_to_usize()).cmp(&other.enum_to_usize())
    }
}

impl PartialOrd for Field {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
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
            println!("{:?}", i.as_ref());
            if ext != "csv" {
                println!("{:?} {:?} != csv: skip..", i.as_ref(), ext);
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
            println!("{:?}", i.as_ref());
            if ext != "xlsx" && ext != "xls" {
                println!("{:?} {:?} != xlsx xls: skip..", i.as_ref(), ext);
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
        println!(">hdr found: {:?}", headers);
        let mut items = Item::default();
        for (i, field) in row.iter().enumerate() {
            //print!("=> {:?}", field);
            if let Some(h) = headers.get(&i) {
                //println!("parse_row --> {:?}", h);
                if let Ok(m) = Field::from_header_and_value(h, field) {
                    if !items.fields.contains(&m) {
                        items.fields.insert(m);
                    }
                }
            } else {
                println!("--> No header {} for {}, skip it", i, field);
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
            if merged.contains_key(&item.unique_id) {
                if let Some(row) = merged.get_mut(&item.unique_id) {
                    /* The rows with same unique id should be merge, so first we
                    get out the Fields that was mergeable */
                    let mut prev = Field::Designator(vec![]);
                    if let Some(curr_designator) = item
                        .fields
                        .iter()
                        .find(|f| matches!(f, Field::Designator(_)))
                    {
                        if let Some(prev_designator) = row
                            .fields
                            .iter()
                            .find(|f| matches!(f, Field::Designator(_)))
                        {
                            curr_designator.expand_designator(prev_designator);
                            prev = prev_designator.clone();
                            row.fields.insert(curr_designator.clone());
                        }
                    }
                    row.fields.remove(&prev);

                    // Merge exta column togheter, in this case we append
                    // value in merged vector
                    for extra in item.fields.iter() {
                        if matches!(extra, Field::Extra(_)) {
                            print!("Merge extra >>> {:?}", extra);
                            if row.fields.contains(extra) {
                                println!(" -> Contiene");
                            } else {
                                row.fields.insert(extra.clone());
                                println!(" -> Insert");
                            }
                        }
                    }
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
        let mut items_table: ItemsTable = ItemsTable::default();
        self.items.sort_by(|a, b| b.category.cmp(&a.category));
        for item in self.items.iter() {
            let mut d = item.fields.clone().into_iter().collect::<Vec<Field>>();
            d.sort();

            let m: Vec<String> = d.iter().map(|f| f.to_string()).collect();

            for f in d.iter() {
                if let Field::Extra(d) = f {
                    if !items_table.headers.contains(&d.hdr) {
                        items_table.headers.push(d.hdr.clone());
                    }
                }
            }
            println!("ItemsTable Headers: {:?}", items_table.headers);

            items_table.rows.push(ItemView {
                quantity: item.quantity,
                unique_id: item.unique_id.clone(),
                category: format!("{}", item.category),
                is_merged: item.is_merged,
                is_np: item.is_np,
                fields: m,
            });
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
    fields: HashSet<Field>,
}

impl Item {
    pub fn default() -> Self {
        let fields = HashSet::new();
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
        let mut category: Category = Category::Invalid;
        for i in self.fields.iter() {
            //println!("{:?}", i);
            if let Field::Designator(d) = i {
                let m = match d.get(0) {
                    Some(m) => m,
                    None => "",
                };

                category = match Regex::new(r"^([a-zA-Z_]{1,3})")
                    .unwrap()
                    .captures(m.to_lowercase().as_str())
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
            };
        }
        self.category = category;
        self.clone()
    }

    fn generate_uuid(&mut self) -> Self {
        let mut description: Field = Field::Description("".to_string());
        let mut comment: Field = Field::Comment("".to_string());
        let footprint: Field = Field::Footprint("".to_string());
        self.is_merged = false;
        self.is_np = false;

        for i in self.fields.iter() {
            match i {
                Field::Description(d) => description = Field::Description(d.clone()),
                Field::Designator(c) => self.quantity = c.len(),
                Field::Comment(c) => comment = Field::Comment(c.clone()),
                _ => (),
            }
        }

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
                self.fields.remove(&comment);
                if Regex::new("^NP ")
                    .unwrap()
                    .is_match(comment.to_string().as_str())
                {
                    comment = Field::Comment("NP Connector".to_string());
                } else {
                    comment = Field::Comment("Connector".to_string());
                }
                self.unique_id = format!(
                    "{:?}-{}-{}-{}-connector",
                    self.category, comment, footprint, description
                );
                self.is_merged = true;
                self.fields.insert(comment);
            }
            Category::Mechanicals => {
                /*
                 * Tactile switch could be have different label, but the component was same
                 */
                if footprint.to_string().to_lowercase().contains("tactile") {
                    self.fields.remove(&comment);
                    self.fields
                        .insert(Field::Comment("Tactile Switch".to_string()));

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
                    self.fields.remove(&comment);
                    self.fields.insert(Field::Comment("Diode LED".to_string()));
                    self.unique_id =
                        format!("{:?}-{}-{}-LED", self.category, footprint, description);
                    self.is_merged = true;
                }
            }
            Category::IC => {
                if footprint.to_string().to_lowercase().contains("rele")
                    || footprint.to_string().to_lowercase().contains("relay")
                {
                    self.fields.remove(&comment);
                    self.fields.insert(Field::Comment("Diode LED".to_string()));
                    self.unique_id =
                        format!("{:?}-{}-{}-relay", self.category, footprint, description);
                    self.is_merged = true;
                }
            }
            _ => {}
        };

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
pub struct ExtraCell {
    hdr: String,
    value: Vec<String>,
}
impl Default for ExtraCell {
    fn default() -> Self {
        ExtraCell {
            hdr: "ExtraCell".to_string(),
            value: vec![],
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum Field {
    Designator(Vec<String>),
    Comment(String),
    Footprint(String),
    Description(String),
    Layer(Vec<String>),
    MountTechnology(Vec<String>),
    Invalid(String),
    Extra(ExtraCell),
}

impl Display for Field {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Designator(v) => write!(f, "{}", v.join(", ")),
            Self::Comment(s) => write!(f, "{}", s),
            Self::Footprint(s) => write!(f, "{}", s),
            Self::Description(s) => write!(f, "{}", s),
            Self::Layer(v) => write!(f, "{}", v.join(", ")),
            Self::MountTechnology(v) => write!(f, "{}", v.join(", ")),
            Self::Extra(s) => write!(f, "{}", s.value.join(", ")),
            Self::Invalid(s) => write!(f, "{}", s),
        }
    }
}

impl Field {
    fn from_header_and_value(header: &str, value: &str) -> Result<Field> {
        let field: Field = match header.to_lowercase().as_str() {
            "designator" => Field::Designator(
                value
                    .to_string()
                    .split(',')
                    .map(|m| m.trim().to_string())
                    .collect(),
            ),
            "comment" => Field::Comment(value.to_string()),
            "footprint" => Field::Footprint(value.to_string()),
            "description" => Field::Description(value.to_string()),
            "mounttechnology" => Field::MountTechnology(vec![value.to_string()]),
            "layer" => Field::Layer(vec![value.to_string()]),
            other => match Regex::new(r"(code|note)\s(.*)").unwrap().captures(other) {
                Some(cc) => match cc.get(0) {
                    Some(s) => {
                        println!(
                            "from_header_and_value: {:?} > h{:?} -> {:?}",
                            s,
                            s.as_str().to_string(),
                            value
                        );
                        Field::Extra(ExtraCell {
                            hdr: s.as_str().to_string(),
                            value: vec![value.to_string().to_uppercase()],
                        })
                    }
                    _ => Field::Invalid(value.to_string()),
                },
                _ => Field::Invalid(value.to_string()),
            },
        };

        if field == Field::Invalid(value.to_string()) {
            bail!("Invalid field {} -> {}", header, value);
        }

        Ok(field)
    }

    pub fn enum_to_usize(&self) -> usize {
        match self {
            Field::Invalid(_) => 0,
            Field::Designator(_) => 1,
            Field::Comment(_) => 2,
            Field::Footprint(_) => 3,
            Field::Description(_) => 4,
            Field::MountTechnology(_) => 5,
            Field::Layer(_) => 6,
            Field::Extra(_) => 7,
        }
    }

    pub fn expand_designator(&self, prev: &Field) -> Field {
        let mut designators: Vec<String> = vec![];
        match prev {
            Field::Designator(d) => designators.extend(d.clone()),
            _ => panic!("expand Wrong field type"),
        };
        match self {
            Field::Designator(d) => designators.extend(d.clone()),
            _ => panic!("Self Wrong field type"),
        };

        Field::Designator(designators)
    }

    pub fn expand_extra(&self, prev: &Field) -> Field {
        let hdr_prev;
        let hdr_self;

        let mut extra: ExtraCell = ExtraCell::default();
        match prev {
            Field::Extra(cell) => {
                hdr_prev = cell.hdr.clone();
                extra.value.extend(cell.value.clone())
            }
            _ => panic!("expand Wrong field type"),
        };
        match self {
            Field::Extra(cell) => {
                hdr_self = cell.hdr.clone();
                extra.value.extend(cell.value.clone())
            }
            _ => panic!("Self Wrong field type"),
        };

        if hdr_prev != hdr_self {
            panic!("ExtraCell header mismatch!");
        }

        Field::Extra(extra)
    }
}
