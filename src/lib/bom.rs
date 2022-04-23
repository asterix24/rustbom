use std::{
    collections::{HashMap, HashSet},
    fmt,
    path::Path,
};

use anyhow::{bail, Result};
use calamine::{open_workbook_auto, DataType, Reader};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

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
            println!("Standard: {}", item);
            Ok(uppercase_first_letter(item))
        }
        "mounttechnology" | "mount_technology" => {
            println!("Standard: {}", item);
            Ok("Mount Technology".to_string())
        }
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
                println!("Found Extra: {}", res);
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

    if item.fields.is_empty() {
        bail!("No fields found");
    }

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
                    fields.insert(Field::IsMerged(true));
                    fields.insert(Field::Category(Category::Connectors));
                }
                Category::Mechanicals => {
                    if footprint.to_lowercase().contains("tactile") {
                        unique_id = format!("{}-{}-tactile", description, footprint);
                        // Overwrite comment, we want merge connectors, that they
                        // could have different comment, so we need to replace it
                        fields.insert(Field::Comment("Tactile Switch".to_string()));
                        fields.insert(Field::IsMerged(true));
                    } else {
                        unique_id = format!("{}-{}-{}", description, comment, footprint);
                        fields.insert(Field::IsNP(false));
                        fields.insert(Field::IsMerged(false));
                    }
                    fields.insert(Field::Category(Category::Mechanicals));
                }
                Category::Diode => {
                    if footprint.to_lowercase().contains("LED") {
                        unique_id = format!("{}-{}-LED", description, footprint);

                        // Overwrite comment, we want merge connectors, that they
                        // could have different comment, so we need to replace it
                        fields.insert(Field::Comment("Diode LED".to_string()));
                        fields.insert(Field::IsMerged(true));
                    } else {
                        unique_id = format!("{}-{}-{}", description, comment, footprint);
                        fields.insert(Field::IsNP(false));
                        fields.insert(Field::IsMerged(false));
                    }
                    fields.insert(Field::Category(Category::Diode));
                }
                Category::IC => {
                    if footprint.to_lowercase().contains("rele")
                        || footprint.to_lowercase().contains("relay")
                    {
                        unique_id = format!("{}-{}-relay", description, footprint);
                        // Overwrite comment, we want merge connectors, that they
                        // could have different comment, so we need to replace it
                        fields.insert(Field::Comment("Relay, Rele\'".to_string()));
                        fields.insert(Field::IsMerged(true));
                    } else {
                        unique_id = format!("{}-{}-{}", description, comment, footprint);
                        fields.insert(Field::IsNP(false));
                        fields.insert(Field::IsMerged(false));
                    }
                    fields.insert(Field::Category(Category::IC));
                }
                others => {
                    unique_id = format!("{}-{}-{}", description, comment, footprint);
                    fields.insert(Field::IsNP(false));
                    fields.insert(Field::IsMerged(false));
                    fields.insert(Field::Category(others.clone()));
                }
            },
            Field::Extra(c) => {
                unique_id = format!("{}-extra-{}", unique_id.clone(), c.1);
            }
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
                println!("Header is empty, skip it");
                continue;
            }

            if let Ok(m) = Field::from_header_and_value(header, field) {
                fields.insert(m);
            }
            if let Ok(m) = Field::category_from_designator(header, field) {
                fields.insert(m);
            }
        }

        let fields_with_uuid = generate_uuid(Item { fields })?;
        println!("**** {:?}", fields_with_uuid);
        Ok(fields_with_uuid)
    }

    pub fn get_items(&self) -> &[Item] {
        &self.items
    }

    pub fn filter(&self, filter: &Category) -> Vec<Item> {
        let mut items = Vec::new();

        for i in self.items.iter() {
            for j in &i.fields {
                if *j == Field::Category(filter.clone()) {
                    items.push(i.clone());
                }
            }
        }

        items
    }

    pub fn collect(&self) -> HashMap<String, Vec<String>> {
        let mut map = HashMap::new();
        for c in Category::iter() {
            let mut items = vec![] as Vec<String>;
            for k in self.filter(&c).iter() {
                let mut s: String = String::new();
                for v in &k.fields {
                    s = format!("{};{}", s, v);
                }
                items.push(s);
            }
            if items.is_empty() {
                continue;
            }
            println!("{:?}: {:#?}", c, items);
            map.insert(format!("{:?}", c), items);
        }
        map
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
    UniqueId(String),
    IsMerged(bool),
    IsNP(bool),
    Category(Category),
    Designator(Vec<String>),
    Comment(String),
    Footprint(String),
    Description(String),
    Extra((String, String)),
    Invalid(String),
}

impl Display for Field {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::UniqueId(s) => write!(f, "{}", s),
            Self::IsMerged(b) => write!(f, "{}", if *b { "Merged" } else { "" }),
            Self::IsNP(b) => write!(f, "{}", if *b { "Not Populate" } else { "" }),
            Self::Category(c) => write!(f, "{:?}", c),
            Self::Designator(v) => write!(f, "{}", v.join(", ")),
            Self::Comment(s) => write!(f, "{}", s),
            Self::Footprint(s) => write!(f, "{}", s),
            Self::Description(s) => write!(f, "{}", s),
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
}

#[cfg(test)]
mod tests {
    use crate::lib::bom;

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
        check_headers.insert(5, "Note Uno".to_string());
        check_headers.insert(6, "Code Due".to_string());
        check_headers.insert(7, "Layer".to_string());
        check_headers.insert(8, "Mount Technology".to_string());
        check_headers.insert(11, "Code Farnell".to_string());
        check_headers.insert(12, "Code Mouser".to_string());
        check_headers.insert(13, "Note Mouser".to_string());
        check_headers.insert(14, "Code Digikey".to_string());

        let mut field = HashSet::new();
        let mut field1 = HashSet::new();
        field.insert(Field::Extra((
            "Note Digi".to_string(),
            "cose varie note".to_string(),
        )));
        field.insert(Field::Footprint("805".to_string()));
        field.insert(Field::Description("x5r".to_string()));
        field.insert(Field::Category(Category::Capacitors));
        field.insert(Field::Extra((
            "Note Produzione".to_string(),
            "cose varie note".to_string(),
        )));
        field.insert(Field::Extra((
            "Code Farnell".to_string(),
            "123".to_string(),
        )));
        field.insert(Field::Designator(vec!["C1".to_string()]));
        field.insert(Field::Extra(("Code Digy".to_string(), "123".to_string())));
        field.insert(Field::Comment("10nF".to_string()));

        field1.insert(Field::Extra(("Code Digy".to_string(), "aa".to_string())));
        field1.insert(Field::Comment("lm2902".to_string()));
        field1.insert(Field::Extra((
            "Note Produzione".to_string(),
            "Aa-bb".to_string(),
        )));
        field1.insert(Field::Footprint("soic".to_string()));
        field1.insert(Field::Description("Op-amp".to_string()));
        field1.insert(Field::Designator(vec!["u3".to_string()]));
        field1.insert(Field::Category(Category::IC));
        field1.insert(Field::Extra(("Code Farnell".to_string(), "aa".to_string())));
        field1.insert(Field::Extra(("Note Digi".to_string(), "Aa-bb".to_string())));

        let bom = Bom::from_xlsx("boms/test.xlsx");

        match bom {
            Ok(bom) => {
                for i in bom.get_items().iter() {
                    println!("{:#?}", i.fields.len());
                    assert_eq!(i.fields.len(), 6);
                    // for j in i.fields.iter() {
                    //     println!("{:#?}", j);
                    //     //assert_eq!(j.to_string(), check_headers.get(&j.field_id()).unwrap());
                    // }
                }
            }
            Err(e) => panic!("Qui..{}", e),
        }
        assert!(false);
    }
}
