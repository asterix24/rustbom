use std::path::Path;

use super::bom::ItemsTable;
use xlsxwriter::prelude::{FormatAlignment, FormatBorder, FormatColor};
use xlsxwriter::{Format, Workbook};

pub struct OutJobXlsx {
    wk: Workbook,
    curr_row: u32,
}

impl OutJobXlsx {
    pub fn new<P: AsRef<Path>>(path: P) -> OutJobXlsx {
        let wk = match Workbook::new(format!("{}.xlsx", path.as_ref().to_str().unwrap()).as_str()) {
            Ok(wk) => wk,
            _ => panic!("Unable to add sheet to open wk"),
        };

        OutJobXlsx { wk, curr_row: 0 }
    }
    pub fn write(mut self, data: &ItemsTable) {
        let mut fmt_default = Format::new();
        fmt_default.set_text_wrap();
        fmt_default.set_font_size(10.0);
        fmt_default.set_text_wrap();

        let mut fmt_header = Format::new();
        fmt_header.set_bg_color(FormatColor::Cyan);
        fmt_header.set_bold();
        fmt_header.set_font_size(12.0);

        let mut fmt_category = Format::new();
        fmt_category.set_bg_color(FormatColor::Yellow);
        fmt_category.set_bold();
        fmt_category.set_border(FormatBorder::Thin);
        fmt_category.set_align(FormatAlignment::CenterAcross);

        let mut fmt_qty = Format::new();
        fmt_qty.set_bg_color(FormatColor::Lime);
        fmt_qty.set_bold();
        fmt_qty.set_font_size(12.0);

        let mut sheet = match self.wk.add_worksheet(None) {
            Ok(wk) => wk,
            _ => panic!("Unable to add sheet to open wk"),
        };

        for (column, hdr) in (0_u16..).zip(data.headers.iter()) {
            sheet
                .write_string(self.curr_row, column, hdr, Some(&fmt_header))
                .unwrap();
        }
        self.curr_row += 1;
        let mut curr_header = "".to_string();
        for i in data.rows.iter() {
            if curr_header != i.category {
                sheet
                    .merge_range(
                        self.curr_row,
                        0,
                        self.curr_row,
                        data.headers.len() as u16,
                        i.category.as_str(),
                        Some(&fmt_category),
                    )
                    .unwrap();
                curr_header = i.category.clone();
                self.curr_row += 1;
            }
            // Write all fields
            for (n, d) in i.fields.iter().enumerate() {
                //debug!("merged {}, np {}", i.is_merged, i.is_np);
                let mut fmt = Some(&fmt_default);
                if n == 0 {
                    fmt = Some(&fmt_qty);
                }
                sheet.write_string(self.curr_row, n as u16, d, fmt).unwrap();
            }

            self.curr_row += 1;
        }
        self.wk.close().unwrap();
    }
}
