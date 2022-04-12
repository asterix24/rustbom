use crate::lib::bom::Bom;
use calamine::{open_workbook_auto, DataType, Reader};

pub struct CsvLoader {
    name: String,
    bom: Bom,
}
//impl Load for CsvLoader {}

pub struct XlsxLoader {
    name: String,
    bom: Bom,
}

impl XlsxLoader {
    pub fn open(filename: &str) -> XlsxLoader {
        let mut bom = Bom::new();

        let mut workbook = match open_workbook_auto(filename) {
            Ok(wb) => wb,
            Err(e) => panic!("{} Error while parsing file {:?}", e, filename),
        };
        let sheet_name = match workbook.sheet_names().first() {
            Some(name) => name.to_string(),
            None => panic!("No sheet found in file {:?}", filename),
        };

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
                        bom.insert_header(column, &s);
                        element.push(s);
                    }
                    bom.insert_row(&element);
                }
            }
            _ => panic!("Male.."),
        }
        Self {
            name: "xlsx".to_string(),
            bom,
        }
    }

    pub fn name(&self) -> &str {
        String::as_str(&self.name)
    }

    pub fn read(&self) -> Bom {
        self.bom.clone()
    }
}
