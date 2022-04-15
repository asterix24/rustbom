use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
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

    fn from_rows_and_headers(rows: &[Vec<String>], headers: &HeaderMap) -> Result<Bom> {
        let items: Vec<_> = rows
            .iter()
            .map(|row| Self::parse_row(row, headers))
            .collect::<Result<_>>()?;

        Ok(Bom { items })
    }

    fn parse_row(row: &[String], headers: &HeaderMap) -> Result<Item> {
        let mut fields = HashSet::new();

        for (i, field) in row.iter().enumerate() {
            let header = headers.get(&i).ok_or_else(|| anyhow!("missing header"))?;
            let field = Field::from_header_and_value(header, field)?;
            fields.insert(field);
        }

        Ok(Item { fields })
    }

    pub fn items(&self) -> &[Item] {
        &self.items
    }
}

pub type HeaderMap = HashMap<usize, String>;

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct Item {
    fields: HashSet<Field>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum Field {
    Quantity(u32),
    Designator(String),
    Comment(String),
    Footprint(String),
    Description(String),
}

impl Field {
    fn from_header_and_value(header: &str, value: &str) -> Result<Field> {
        Ok(match header {
            "Quantity" => Field::Quantity(value.parse()?),
            "Designator" => Field::Designator(value.to_string()),
            "Comment" => Field::Comment(value.to_string()),
            "Footprint" => Field::Footprint(value.to_string()),
            "Description" => Field::Description(value.to_string()),
            _ => bail!("unknown header"),
        })
    }
}
