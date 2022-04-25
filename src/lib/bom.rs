use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
    fmt,
    path::Path,
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
            println!("Standard: {}", item);
            Ok(uppercase_first_letter(item))
        }
        "mounttechnology" | "mount_technology" | "MountTechnology" => {
            println!("Standard: {}", item);
            Ok("MountTechnology".to_string())
        }
        _ => {
            let res: String;
            match re_note.captures(item.to_lowercase().as_ref()) {
                Some(cc) => match cc.get(0) {
                    Some(s) => res = s.as_str().to_string().to_uppercase(),
                    _ => bail!("Invalid header key: {}", item),
                },
                _ => bail!("Invalid header key: {}", item),
            }
            Ok(res)
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

    let mut category = "".to_string();
    let mut description = "".to_string();
    let mut comment = "".to_string();
    let mut footprint = "".to_string();
    let mut unique_id = "".to_string();
    let mut fields = HashSet::new();
    let mut qty = 0;

    if item.fields.is_empty() {
        bail!("No fields found");
    }

    for i in item.fields.iter() {
        match i {
            Field::Category(d) => category = format!("{:?}", d),
            Field::Description(d) => description = d.clone(),
            Field::Designator(d) => qty = d.len(),
            Field::Comment(c) => comment = c.clone(),
            Field::Footprint(f) => footprint = f.clone(),
            _ => (),
        }
    }

    for v in item.fields.iter() {
        match v {
            Field::Quantity(_) => {
                fields.insert(Field::Quantity(qty));
            }
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
                    unique_id = format!(
                        "{}-{}-{}-{}-connector",
                        category, comment, footprint, description
                    );

                    // Overwrite comment, we want merge connectors, that they
                    // could have different comment, so we need to replace it
                    fields.insert(Field::Comment(comment.clone()));
                    fields.insert(Field::IsMerged(true));
                    fields.insert(Field::Category(Category::Connectors));
                }
                Category::Mechanicals => {
                    if footprint.to_lowercase().contains("tactile") {
                        unique_id = format!("{}-{}-{}-tactile", category, footprint, description);
                        // Overwrite comment, we want merge connectors, that they
                        // could have different comment, so we need to replace it
                        fields.insert(Field::Comment("Tactile Switch".to_string()));
                        fields.insert(Field::IsMerged(true));
                    } else {
                        unique_id =
                            format!("{}-{}-{}-{}", category, comment, footprint, description);
                        fields.insert(Field::IsNP(false));
                        fields.insert(Field::IsMerged(false));
                    }
                    fields.insert(Field::Category(Category::Mechanicals));
                }
                Category::Diode => {
                    if footprint.to_lowercase().contains("LED") {
                        unique_id = format!("{}-{}-{}-LED", category, footprint, description);

                        // Overwrite comment, we want merge connectors, that they
                        // could have different comment, so we need to replace it
                        fields.insert(Field::Comment("Diode LED".to_string()));
                        fields.insert(Field::IsMerged(true));
                    } else {
                        unique_id =
                            format!("{}-{}-{}-{}", category, comment, footprint, description);
                        fields.insert(Field::IsNP(false));
                        fields.insert(Field::IsMerged(false));
                    }
                    fields.insert(Field::Category(Category::Diode));
                }
                Category::IC => {
                    if footprint.to_lowercase().contains("rele")
                        || footprint.to_lowercase().contains("relay")
                    {
                        unique_id = format!("{}-{}-{}-relay", category, footprint, description);
                        // Overwrite comment, we want merge connectors, that they
                        // could have different comment, so we need to replace it
                        fields.insert(Field::Comment("Relay, Rele\'".to_string()));
                        fields.insert(Field::IsMerged(true));
                    } else {
                        unique_id =
                            format!("{}-{}-{}-{}", category, comment, footprint, description);
                        fields.insert(Field::IsNP(false));
                        fields.insert(Field::IsMerged(false));
                    }
                    fields.insert(Field::Category(Category::IC));
                }
                others => {
                    println!(".. Category: {:?}", others);
                    unique_id = format!("{}-{}-{}-{}", category, comment, footprint, description);
                    fields.insert(Field::IsNP(false));
                    fields.insert(Field::IsMerged(false));
                    fields.insert(Field::Category(others.clone()));
                }
            },
            other => {
                fields.insert(other.clone());
            }
        }
    }
    fields.insert(Field::UniqueId(unique_id));

    println!("len {:?}", fields.len());
    Ok(Item { fields })
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
        let (rows, headers) = xlsx_loader(path);
        Bom::from_rows_and_headers(&rows, &headers)
    }

    fn from_rows_and_headers(rows: &[Vec<String>], headers: &HeaderMap) -> Result<Bom> {
        let mut items: Vec<_> = Vec::new();
        for row in rows.iter() {
            if let Ok(item) = Self::parse_row(row, headers) {
                items.push(item);
            }
        }
        Ok(Bom { items })
    }

    fn parse_row(row: &[String], headers: &HeaderMap) -> Result<Item> {
        let mut fields = Item::default();
        for (i, field) in row.iter().enumerate() {
            let header = match headers.get(&i) {
                Some(h) => h,
                None => {
                    println!("No header for {}, skip it", i);
                    ""
                }
            };

            if let Ok(m) = Field::category_from_designator(header, field) {
                fields.insert(m);
            }

            if let Ok(m) = Field::from_header_and_value(header, field) {
                fields.insert(m);
            }
        }

        let mut fields_with_uuid = generate_uuid(Item { fields })?;
        fields_with_uuid.clean();
        println!("**** {:?}", fields_with_uuid);
        Ok(fields_with_uuid)
    }

    pub fn get_items(&self) -> &[Item] {
        &self.items
    }

    pub fn filter(&self, filter: Field) -> Vec<Item> {
        let mut items = Vec::new();
        for i in self.items.iter() {
            for j in &i.fields {
                if *j == filter {
                    items.push(i.clone());
                }
            }
        }
        items
    }

    pub fn merge(&self) -> Bom {
        let mut merged: HashMap<Field, Item> = HashMap::new();
        for v in &self.items {
            for i in v.fields.iter() {
                if let Field::UniqueId(uuid) = i {
                    if merged.contains_key(&Field::UniqueId(uuid.clone())) {
                        if let Err(e) = merged.get(&Field::UniqueId(uuid.clone())).unwrap().sum(v) {
                            println!("{}", e);
                        }
                    } else {
                        merged.insert(i.clone(), v.clone());
                    }
                }
            }
        }

        Bom {
            items: merged.values().cloned().collect(),
        }
    }

    pub fn collect(&self) -> Vec<Vec<String>> {
        let mut items: Vec<Vec<String>> = vec![];

        for i in self.items.iter() {
            let mut field: Vec<Field> = i.fields.iter().cloned().collect();
            field.sort();
            let mut item: Vec<String> = vec![];
            for j in field.iter() {
                item.push(j.to_string());
            }
            items.push(item);
        }
        items
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

impl Item {
    pub fn default() -> HashSet<Field> {
        let mut fields = HashSet::new();
        fields.insert(Field::Quantity(0));
        fields.insert(Field::Category(Category::Invalid));
        fields.insert(Field::Designator(vec![] as Vec<String>));
        fields.insert(Field::Comment("".to_string()));
        fields.insert(Field::Footprint("".to_string()));
        fields.insert(Field::Layer(vec![] as Vec<String>));
        fields.insert(Field::MountTechnology(vec![] as Vec<String>));
        fields.insert(Field::Extra(("".to_string(), "".to_string())));

        fields
    }

    pub fn clean(&mut self) -> Item {
        self.fields.remove(&Field::Quantity(0));
        self.fields.remove(&Field::Category(Category::Invalid));
        self.fields
            .remove(&Field::Designator(vec![] as Vec<String>));
        self.fields.remove(&Field::Comment("".to_string()));
        self.fields.remove(&Field::Footprint("".to_string()));
        self.fields.remove(&Field::Layer(vec![] as Vec<String>));
        self.fields
            .remove(&Field::MountTechnology(vec![] as Vec<String>));
        self.fields
            .remove(&Field::Extra(("".to_string(), "".to_string())));

        Item {
            fields: self.fields.clone(),
        }
    }

    pub fn sum(&self, a: &Item) -> Result<Item> {
        let mut fields = HashSet::new();
        let mut qty: usize = 0;
        let mut designators: Vec<String> = vec![];
        let mut layer: Vec<String> = vec![];
        let mut mount: Vec<String> = vec![];

        for a in a.fields.iter() {
            match a {
                Field::Quantity(q) => {
                    qty = *q;
                }
                Field::Designator(d) => {
                    designators.append(&mut d.clone());
                }
                Field::Layer(d) => {
                    layer.append(&mut d.clone());
                }
                Field::MountTechnology(d) => {
                    mount.append(&mut d.clone());
                }
                _ => {}
            }
        }

        for field in self.fields.iter() {
            match field {
                Field::Quantity(q) => {
                    fields.insert(Field::Quantity(qty + *q));
                }
                Field::Designator(d) => {
                    designators.append(&mut d.clone());
                    fields.insert(Field::Designator(designators.clone()));
                }
                Field::Layer(d) => {
                    layer.append(&mut d.clone());
                    fields.insert(Field::Layer(layer.clone()));
                }
                Field::MountTechnology(d) => {
                    mount.append(&mut d.clone());
                    fields.insert(Field::MountTechnology(mount.clone()));
                }
                other => {
                    fields.insert(other.clone());
                }
            }
        }
        Ok(Item { fields })
    }
    pub fn collect(&self) -> Vec<Field> {
        let mut items: Vec<Field> = vec![];

        for i in self.fields.iter() {
            items.push(i.clone());
        }

        items.sort();
        items
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

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum Field {
    Quantity(usize),
    UniqueId(String),
    IsMerged(bool),
    IsNP(bool),
    Category(Category),
    Designator(Vec<String>),
    Comment(String),
    Footprint(String),
    Description(String),
    Layer(Vec<String>),
    MountTechnology(Vec<String>),
    Extra((String, String)),
    Invalid(String),
}

impl Display for Field {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Quantity(q) => write!(f, "{}", q),
            Self::UniqueId(s) => write!(f, "{}", s),
            Self::IsMerged(b) => write!(f, "{}", if *b { "Merged" } else { "" }),
            Self::IsNP(b) => write!(f, "{}", if *b { "Not Populate" } else { "" }),
            Self::Category(c) => write!(f, "{:?}", c),
            Self::Designator(v) => write!(f, "{}", v.join(", ")),
            Self::Comment(s) => write!(f, "{}", s),
            Self::Footprint(s) => write!(f, "{}", s),
            Self::Description(s) => write!(f, "{}", s),
            Self::Layer(v) => write!(f, "{}", v.join(", ")),
            Self::MountTechnology(v) => write!(f, "{}", v.join(", ")),
            Self::Extra((_, s)) => write!(f, "{}", s),
            Self::Invalid(s) => write!(f, "{}", s),
        }
    }
}

impl Field {
    fn from_header_and_value(header: &str, value: &str) -> Result<Field> {
        let re_note = Regex::new(r"note\s(.*)").unwrap();
        let re_code = Regex::new(r"code\s(.*)").unwrap();

        let field: Field = match header {
            "Designator" => Field::Designator(
                value
                    .to_string()
                    .split(',')
                    .map(|m| m.trim().to_string())
                    .collect(),
            ),
            "Comment" => Field::Comment(value.to_string()),
            "Footprint" => Field::Footprint(value.to_string()),
            "Description" => Field::Description(value.to_string()),
            other => {
                let mut extra = String::new();
                if let Some(cc) = re_code.captures(other.to_lowercase().as_ref()) {
                    if let Some(m) = cc.get(1).map(|m| m.as_str()) {
                        extra = m.to_string();
                    }
                }
                if let Some(cc) = re_note.captures(other.to_lowercase().as_ref()) {
                    if let Some(m) = cc.get(1).map(|m| m.as_str()) {
                        extra = m.to_string();
                    }
                }
                if !extra.is_empty() {
                    Field::Extra((other.to_string(), value.to_string()))
                } else {
                    Field::Invalid("-".to_string())
                }
            }
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

    fn enum_to_usize(&self) -> usize {
        match self {
            Field::Invalid(_) => 0,
            Field::UniqueId(_) => 1,
            Field::IsMerged(_) => 2,
            Field::IsNP(_) => 3,
            Field::Category(_) => 4,
            Field::Quantity(_) => 5,
            Field::Designator(_) => 6,
            Field::Comment(_) => 7,
            Field::Footprint(_) => 8,
            Field::Description(_) => 9,
            Field::MountTechnology(_) => 10,
            Field::Layer(_) => 11,
            Field::Extra(x) => 12 + x.0.len() + x.1.len(),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    #[test]
    fn test_xlsx_loader() {
        let (_, headers) = xlsx_loader("boms/bom.xlsx");
        let mut check_headers: HeaderMap = HashMap::new();

        check_headers.insert(1, "Designator".to_string());
        check_headers.insert(2, "Comment".to_string());
        check_headers.insert(3, "Footprint".to_string());
        check_headers.insert(4, "Description".to_string());
        check_headers.insert(5, "Note Uno".to_string());
        check_headers.insert(6, "Code Due".to_string());
        check_headers.insert(7, "Layer".to_string());
        check_headers.insert(8, "Mount Technology".to_string());
        check_headers.insert(11, "Code Farnell".to_string());
        check_headers.insert(12, "Code Mouser".to_string());
        check_headers.insert(13, "Note Mouser".to_string());
        check_headers.insert(14, "Code Digikey".to_string());

        println!("{:#?}", check_headers);
        println!("{:#?}", headers);
        for i in check_headers.iter() {
            println!("{}", i.1);
            assert_eq!(i.1, headers.get(i.0).unwrap());
        }
    }
    #[test]
    fn test_parse_row() {
        let mut check_headers: HeaderMap = HashMap::new();

        check_headers.insert(1, "Designator".to_string());
        check_headers.insert(2, "Comment".to_string());
        check_headers.insert(3, "Footprint".to_string());
        check_headers.insert(4, "Description".to_string());
        check_headers.insert(7, "Layer".to_string());
        check_headers.insert(8, "MountTechnology".to_string());
        check_headers.insert(11, "Code Farnell".to_string());
        check_headers.insert(12, "Code Mouser".to_string());
        check_headers.insert(13, "Note Mouser".to_string());
        check_headers.insert(14, "Code Digikey".to_string());

        /*
        "".to_string(), //"Designator"
        "".to_string(), //"Comment"
        "".to_string(), //"Footprint"
        "".to_string(), //"Description"
        "".to_string(), //"Layer"
        "".to_string(), //"MountTechnology"
        "".to_string(), //"Code Farnell"
        "".to_string(), //"Code Mouser"
        "".to_string(), //"Note Mouser"
        "".to_string(), //"Code Digikey"
         */

        let rows: Vec<Vec<String>> = vec![
            vec![
                "C0,C1,C2".to_string(), //"Designator"
                "100nF".to_string(),    //"Comment"
                "0603".to_string(),     //"Footprint"
                "Ceramic".to_string(),  //"Description"
                "Top".to_string(),      //"Layer"
                "SMD".to_string(),      //"MountTechnology"
                "1245".to_string(),     //"Code Farnell"
                "123-aa".to_string(),   //"Code Mouser"
                "".to_string(),         //"Note Mouser"
                "123-nd".to_string(),   //"Code Digikey"
            ],
            vec![
                "R0,R1,R2".to_string(), //"Designator"
                "100R".to_string(),     //"Comment"
                "0603".to_string(),     //"Footprint"
                "Resistor".to_string(), //"Description"
                "Bottom".to_string(),   //"Layer"
                "SMD".to_string(),      //"MountTechnology"
                "1245".to_string(),     //"Code Farnell"
                "123-aa".to_string(),   //"Code Mouser"
                "".to_string(),         //"Note Mouser"
                "123-nd".to_string(),   //"Code Digikey"
            ],
        ];

        let rows_checks: Vec<Vec<Field>> = vec![vec![
            Field::Quantity(3),                    //"Designator"
            Field::UniqueId("".to_string()),       //"Comment"
            Field::IsMerged(false),                //"Footprint"
            Field::IsNP(false),                    //"Description"
            Field::Category(Category::Capacitors), //"Layer"
            Field::Designator(vec!["C0".to_string(), "C1".to_string(), "C2".to_string()]), //"MountTechnology"
            Field::Comment("100nF".to_string()),  //"Code Farnell"
            Field::Footprint("0603".to_string()), //"Code Mouser"
            Field::Description("Ceramic".to_string()), //"Note Mouser"
            Field::Layer(vec!["Top".to_string()]), //"Code Digikey"
            Field::MountTechnology(vec!["SMD".to_string()]), //"Code Digikey"
            Field::Extra(("Code Farnell".to_string(), "".to_string())), //"Code Digikey"
            Field::Extra(("Code Mouser".to_string(), "".to_string())), //"Code Digikey"
            Field::Extra(("Note Mouser".to_string(), "".to_string())), //"Code Digikey"
            Field::Extra(("Code Digikey".to_string(), "".to_string())), //"Code Digikey"
        ]];

        for (n, row) in rows.iter().enumerate() {
            if let Ok(item) = Bom::parse_row(row, &check_headers) {
                println!("{:#?}", item.collect());
                assert_eq!(item.fields.len(), 13);
                for (k, i) in item.collect().iter().enumerate() {
                    assert_eq!(i, rows_checks[n].get(k).unwrap());
                }
            }
        }
        assert!(false);
    }
    #[test]
    fn test_is_header() {
        let data_in: Vec<String> = vec![
            "Designator".to_string(),
            "Comment".to_string(),
            "Footprint".to_string(),
            "Description".to_string(),
            "No Uno".to_string(),
            "Note Uno Due Tre".to_string(),
            "Code Due".to_string(),
            "Layer".to_string(),
            "mounttechnology".to_string(),
            "mount_technology".to_string(),
            "MountTechnology".to_string(),
            "Code Farnell".to_string(),
            "Code Mouser".to_string(),
            "Note Mouser".to_string(),
            "Code Digikey".to_string(),
            "Note Digi".to_string(),
            "Node 1223".to_string(),
        ];
        let data_check: Vec<String> = vec![
            "Designator".to_string(),
            "Comment".to_string(),
            "Footprint".to_string(),
            "Description".to_string(),
            "Invalid header key: No Uno".to_string(),
            "NOTE UNO DUE TRE".to_string(),
            "CODE DUE".to_string(),
            "Layer".to_string(),
            "MountTechnology".to_string(),
            "MountTechnology".to_string(),
            "MountTechnology".to_string(),
            "CODE FARNELL".to_string(),
            "CODE MOUSER".to_string(),
            "NOTE MOUSER".to_string(),
            "CODE DIGIKEY".to_string(),
            "NOTE DIGI".to_string(),
            "Invalid header key: Node 1223".to_string(),
        ];

        for (n, i) in data_in.iter().enumerate() {
            match is_header_key(i.as_str()) {
                Ok(s) => {
                    println!("Ok: {}", s);
                    assert_eq!(s, data_check[n]);
                }
                Err(e) => {
                    println!("Fail: {}", e);
                    assert_eq!(format!("{}", e), data_check[n]);
                }
            }
        }
        assert!(false);
    }
}
