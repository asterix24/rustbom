use std::fmt;
use std::collections::HashMap;


#[derive(Debug, PartialEq, PartialOrd, Clone, Eq, Ord)]
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


#[derive(Debug, PartialEq, PartialOrd, Clone, Eq, Ord, Copy)]
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

#[derive(Debug, Clone)]
pub struct ExtraCol {
    pub label: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct Item {
    unique_id: String,
    category: Category,
    designator: String,
    footprint: String,
    description: String,
    comment: String,
    extra: Vec<ExtraCol>,
}
impl Item {
    pub fn new(
    ) -> Self {
        let local_item = Self {
            unique_id: "".to_string(),
            category: Category::IVALID,
            designator: "".to_string(),
            footprint: "".to_string(),  
            description: "".to_string(),
            comment: "".to_string(),
            extra: vec![],
        };
        local_item
    }

    pub fn push_extra(&mut self, extra: ExtraCol) {
        self.extra.push(extra);
    }
    pub fn get_unique_id(&self) -> String {
        self.unique_id.clone()
    }
    pub fn get_category(&self) -> Category {
        self.category.clone()
    }
    pub fn get_designator(&self) -> String {
        self.designator.clone()
    }
    pub fn get_footprint(&self) -> String {
        self.footprint.clone()
    }
    pub fn get_description(&self) -> String {
        self.description.clone()
    }
    pub fn get_comment(&self) -> String {
        self.comment.clone()
    }
    pub fn get_extra(&self) -> Vec<ExtraCol> {
        self.extra.clone()
    }
    pub fn push(&self, row: &Vec<String>, header: &HashMap<usize, Header>) {
        for (n, field) in row.iter().enumerate() {
            tide::log::info!("{} {} {:?}", n, field, header.get(&n).unwrap_or(&Header::INVALID));
        }
    }
}

