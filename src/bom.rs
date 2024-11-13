use anyhow::{bail, Result};
use calamine::{open_workbook_auto, DataType, Reader};
use log::{debug, info, warn};
use rand::{Rng, SeedableRng};
use rand_pcg::Pcg32;
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

impl Display for ItemView {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let fields: String = self.fields.join(";");
        write!(
            f,
            "{:};{:};{:};{:};{:}",
            self.unique_id, self.is_merged, self.is_np, self.category, fields
        )
    }
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

const STD_HEADERS: [&str; 7] = [
    "quantity",
    "designator",
    "comment",
    "footprint",
    "description",
    "layer",
    "mounttechnology",
];

pub fn merge_key_list() -> Vec<String> {
    let mut keys: Vec<String> = Vec::new();
    for i in STD_HEADERS {
        keys.push(i.to_string());
    }

    keys
}

impl Bom {
    pub fn loader<P: AsRef<Path>>(path: &[P], merge_keys: &[String]) -> Bom {
        let mut it1: Vec<Item> = Vec::new();
        if let Ok(i) = Bom::from_csv(path, merge_keys) {
            it1 = i;
        }

        let mut it2: Vec<Item> = Vec::new();
        if let Ok(i) = Bom::from_xlsx(path, merge_keys) {
            it2 = i;
        }

        it1.extend(it2);
        Bom { items: it1 }
    }

    pub fn from_csv<P: AsRef<Path>>(path: &[P], merge_keys: &[String]) -> Result<Vec<Item>> {
        let mut items: Vec<_> = Vec::new();

        for i in path.iter() {
            let mut rng = Pcg32::seed_from_u64(i.as_ref().to_path_buf().capacity() as u64);
            let ext = Path::new(i.as_ref())
                .extension()
                .and_then(OsStr::to_str)
                .unwrap();
            if ext != "csv" {
                warn!("{:?} {:?} != csv: skip..", i.as_ref(), ext);
                continue;
            }
            let (rows, headers) = csv_loader(i.as_ref());
            if let Ok(mut ii) = Bom::from_rows_and_headers(&rows, &headers, merge_keys, &mut rng) {
                items.append(&mut ii);
            }
        }
        Ok(items)
    }

    pub fn from_xlsx<P: AsRef<Path>>(path: &[P], merge_keys: &[String]) -> Result<Vec<Item>> {
        let mut items: Vec<_> = Vec::new();

        for i in path.iter() {
            let mut rng = Pcg32::seed_from_u64(i.as_ref().to_path_buf().capacity() as u64);
            let ext = Path::new(i.as_ref())
                .extension()
                .and_then(OsStr::to_str)
                .unwrap();
            if ext != "xlsx" && ext != "xls" {
                warn!("{:?} {:?} != xlsx xls: skip..", i.as_ref(), ext);
                continue;
            }
            let (rows, headers) = xlsx_loader(i);
            if let Ok(mut ii) = Bom::from_rows_and_headers(&rows, &headers, merge_keys, &mut rng) {
                items.append(&mut ii);
            }
        }
        Ok(items)
    }

    fn from_rows_and_headers(
        rows: &[Vec<String>],
        headers: &HeaderMap,
        merge_keys: &[String],
        seed: &mut Pcg32,
    ) -> Result<Vec<Item>> {
        let mut items: Vec<_> = Vec::new();
        for row in rows.iter() {
            if let Ok(item) = Self::parse_row(row, headers, merge_keys, seed) {
                items.push(item);
            }
        }
        Ok(items)
    }

    fn parse_row(
        row: &[String],
        headers: &HeaderMap,
        merge_keys: &[String],
        seed: &mut Pcg32,
    ) -> Result<Item> {
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
        Ok(items.guess_category().generate_uuid(merge_keys, seed))
    }

    pub fn merge(&self) -> Bom {
        /*
        Merge policy:
        All Item element was merged by unique_id, if two row have same unique_id we could merge it in one, but:
        - if NP, skip it
        - designators is put all togheter in vector
        - quantity is increased
        */
        let mut merged: HashMap<String, Item> = HashMap::new();
        for item in self.items.iter() {
            print!("ID-> {:?}\n", item);
            if let Some(prev) = merged.get_mut(&item.unique_id) {
                /*
                 * We found a row with same unique_id, so will go to merge.
                 * First we start with designator, and update also the quantity.
                 */
                if let Some(Field::List(dd)) = prev.fields.get_mut("designator") {
                    if let Some(Field::List(last_dd)) = item.fields.get("designator") {
                        dd.extend(last_dd.clone());
                    }
                    dd.sort();
                    dd.dedup();
                    prev.quantity = dd.len();
                }

                /*
                 * Parse Filed vector, to merge columns
                 */
                for c in item.fields.keys() {
                    // If in field we found a heder we skip it
                    if STD_HEADERS.contains(&c.as_str()) {
                        continue;
                    }
                    if let Some(Field::List(dd)) = prev.fields.get_mut(c) {
                        if let Some(Field::List(last_dd)) = item.fields.get(c) {
                            dd.extend(last_dd.clone());
                        }
                        dd.sort();
                        dd.dedup();
                    } else if let Some(m) = item.fields.get(c) {
                        prev.fields.insert(c.clone(), m.clone());
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
        let mut headers: HashMap<String, usize> = HashMap::new();
        for (i, h) in STD_HEADERS.iter().enumerate() {
            info!("mappa->{} {}", i, h);
            headers.insert(h.to_string(), i);
        }

        // Get header map and row max len
        let mut row_capacity: usize = headers.len();
        for item in self.items.iter() {
            for hdr in item.fields.iter() {
                if !headers.contains_key(hdr.0) {
                    headers.insert(hdr.0.clone(), row_capacity);
                    row_capacity += 1;
                }
                // info!(
                //     "Header MAP -> {} {} {}",
                //     row_capacity, headers[hdr.0], hdr.0
                // );
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
            m[0] = format!("{}", item.quantity);

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

    fn generate_uuid(&mut self, merge_keys: &[String], seed: &mut Pcg32) -> Self {
        self.unique_id = "".to_string();
        self.is_merged = false;
        self.is_np = false;

        // Update quantity counting the designator elements
        if let Some(d) = self.fields.get("designator") {
            self.quantity = match d {
                Field::List(l) => l.len(),
                _ => 0,
            };
        };

        // No keys mergs, so get all items
        if merge_keys.is_empty() {
            for _ in 0..15 {
                self.unique_id = format!("{}{:}", self.unique_id, seed.gen_range(0..9));
            }
            return self.clone();
        }

        // Vector of keys to merge
        let mut mm: Vec<String> = vec![];
        mm.push(format!("{}", self.category));

        // Ckeck if line is NP
        for item in merge_keys.iter() {
            if let Some(d) = self.fields.get(item) {
                // anyway the NP mark should not merge
                if Regex::new("^NP ").unwrap().is_match(d.to_string().as_str()) {
                    self.is_np = true;
                    println!("np {:?}", d);
                }
            }
        }

        for item in merge_keys.iter() {
            if let Some(d) = self.fields.get(item) {
                let mut field: String = format!("{}", d.clone());
                match self.category {
                    Category::Connectors => {
                        if "comment" == item {
                            field = String::from("Connector");
                            if self.is_np {
                                field = String::from("NP Connector");
                            }

                            self.is_merged = true;
                            self.fields.remove("comment");
                            self.fields
                                .insert(String::from("comment"), Field::Item(field.clone()));
                        }
                    }
                    Category::Diode => {
                        if "footprint" == item {
                            if field.contains("LED") {
                                field = String::from("LED");
                                if self.is_np {
                                    field = String::from("NP LED");
                                }
                            }

                            self.is_merged = true;
                            self.fields.remove("comment");
                            self.fields
                                .insert(String::from("comment"), Field::Item(field.clone()));
                        }
                    }
                    _ => (),
                }
                println!(">>>>>>>>>{}", field);
                mm.push(field);
            };
        }

        // generate_uuid
        self.unique_id = mm.join("-");
        println!("unique ID -> {:} {:?}", self.unique_id, self.fields);
        self.clone()
    }
}

impl Default for Item {
    fn default() -> Self {
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
                if let Some(x) = m.first() {
                    x.to_lowercase()
                } else {
                    "".to_string()
                }
            }
            _ => "".to_string(),
        }
    }

    fn from_header_and_value(header: &str, value: &str) -> Result<(String, Field)> {
        let mut hdr = header.to_lowercase();
        let field: Field = match hdr.as_str() {
            "designator" => Field::List(value.split(',').map(|m| m.trim().to_string()).collect()),
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

//#[cfg(test)]
//mod tests {
//    use super::*;
//    #[test]
//    fn test_merge1() -> Result<(), String> {
//        let files = ["tests/data/test0.xlsx"];
//        let keys = [
//            "Quantity",
//            "Designator",
//            "Comment",
//            "Footprint",
//            "Description",
//        ];
//        let bom = Bom::loader(files.as_slice(), &keys.map(String::from));
//        let data = bom.merge().odered_vector_table();
//        print!("{:?}", data.headers);
//        print!("{:?}", data.rows);
//
//        Ok(())
//    }
//}
