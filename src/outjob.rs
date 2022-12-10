use std::path::Path;

use super::bom::ItemsTable;
use xlsxwriter::*;

pub struct OutJobXlsx {
    wk: Workbook,
    curr_row: u32,
}

impl OutJobXlsx {
    pub fn new<P: AsRef<Path>>(path: P) -> OutJobXlsx {
        OutJobXlsx {
            wk: Workbook::new(path.as_ref().to_str().unwrap()),
            curr_row: 0,
        }
    }
    pub fn write(mut self, data: &ItemsTable) {
        let fmt_defalt = self
            .wk
            .add_format()
            .set_text_wrap()
            .set_font_size(10.0)
            .set_text_wrap();
        let fmt_header = self
            .wk
            .add_format()
            .set_bg_color(FormatColor::Cyan)
            .set_bold()
            .set_font_size(12.0);
        let fmt_category = self
            .wk
            .add_format()
            .set_bg_color(FormatColor::Yellow)
            .set_bold()
            .set_border(FormatBorder::Thin)
            .set_align(FormatAlignment::CenterAcross);
        let fmt_qty = self
            .wk
            .add_format()
            .set_bg_color(FormatColor::Lime)
            .set_bold()
            .set_font_size(12.0);

        let mut sheet = match self.wk.add_worksheet(None) {
            Ok(wk) => wk,
            _ => panic!("Unable to add sheet to open wk"),
        };

        for (column, hdr) in (0_u16..).zip(data.headers.iter()) {
            sheet
                .write_string(self.curr_row, column as u16, hdr, Some(&fmt_header))
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
                let mut fmt = Some(&fmt_defalt);
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
