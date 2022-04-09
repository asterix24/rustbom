use std::collections::HashMap;
use std::hash::Hash;

use calamine::{open_workbook_auto, DataType, Reader, Sheets};

use crate::lib::utils::is_header_key;
use crate::lib::item::Header;

pub struct CsvLoader {
    name: String,
    data: Vec<Vec<String>>,
}

pub struct XlsxLoader {
    name: String,
    workbook: Sheets,
    sheet_name: String,
    data: Vec<Vec<String>>,
    header_map: HashMap<usize, Header>,
}
pub trait Load {
    fn new(name: &'static str) -> Self;
    fn name(&self) -> &str;
    fn read(&mut self);
    fn raw_data(&self) -> &Vec<Vec<String>>;
    fn map_data(&self) -> &HashMap<usize, Header>;
}

//impl Load for CsvLoader {}

impl Load for XlsxLoader {
    fn new(filename: &str) -> XlsxLoader {
        let sheet_name: String;
        let workbook;

        workbook = match open_workbook_auto(filename) {
            Ok(wb) => wb,
            Err(e) => panic!("{} Error while parsing file {:?}", e, filename),
        };
        sheet_name = match workbook.sheet_names().first() {
            Some(name) => name.to_string(),
            None => panic!("No sheet found in file {:?}", filename),
        };

        XlsxLoader {
            name: "xlsx".to_string(),
            workbook,
            sheet_name,
            data: Vec::new(),
            header_map: HashMap::new(),
        }
    }

    fn name(&self) -> &str {
        String::as_str(&self.name)
    }

    fn read(&mut self){
        match self.workbook.worksheet_range(self.sheet_name.as_str()) {
            Some(Ok(range)) => {
                let (rw, cl) = range.get_size();
                for row in 0..rw {
                    let mut element: Vec<String> = Vec::new();
                    for column in 0..cl {

                        let s = match range.get((row, column)) {
                            Some(DataType::String(s)) => s.to_string(),
                            Some(DataType::Int(s)) => s.to_string(),
                            Some(DataType::Float(s)) => s.to_string(),
                            _ => "-".to_string(),
                        };
                        let h = is_header_key(&s);
                        if h != Header::INVALID {
                            self.header_map.insert(column, h);
                            continue;
                        }
                        element.push(s);
                    }
                    if element.len() > 0 {
                        self.data.push(element);
                    }
                }
            }
            _ => panic!("Male.."),
        }
    }

    fn raw_data(&self) -> &Vec<Vec<String>> {
        &self.data
    }

    fn map_data(&self) -> &HashMap<usize, Header> {
        &self.header_map
    }
}
