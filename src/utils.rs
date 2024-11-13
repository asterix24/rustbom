use lazy_static::lazy_static;
use regex::Regex;

pub fn detect_measure_unit(comment: &str) -> String {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"^([KkR|C|L|Y])").unwrap();
    }
    match RE.captures(comment.as_ref()) {
        None => String::from("unknow"),
        Some(cc) => match cc.get(1).map_or("", |m| m.as_str()) {
            "K" | "k" | "R" => String::from("ohm"),
            "C" => String::from("F"),
            "L" => String::from("H"),
            "Y" => String::from("Hz"),
            _ => String::from("missing"),
        },
    }
}

pub fn value_to_eng_notation(base: f32, exp: i32, unit: &str) -> String {
    let unitletter = match exp {
        12 => "G",
        6 => "M",
        3 => "k",
        0 | 1 => "",
        -3 => "m",
        -6 => "u",
        -9 => "n",
        -12 => "p",
        _ => panic!("Invalid exp for conversion"),
    };

    if base < 0.0 {
        return String::from("NP");
    }

    let mut value = format!("{}", base);
    if unit == "ohm" {
        if value.contains('.') {
            value = match unitletter {
                "G" | "M" | "k" => value.replace('.', unitletter),
                _ => format!("{}R", value),
            }
        } else {
            value = match unitletter {
                "G" | "M" | "k" => format!("{}{}", value, unitletter),
                "m" | "u" | "n" | "p" => format!("{}{}{}", value, unitletter, unit),
                _ => format!("{}R", value),
            }
        }
    } else {
        value = format!("{}{}{}", value, unitletter, unit);
    }
    value
}

pub fn convert_comment_to_value(comment: &str) -> (f32, i32) {
    if comment == "NP" {
        return (-1.0, 0);
    }

    let v = comment
        .split(',')
        .map(|item| item.trim())
        .collect::<Vec<_>>();

    let value = match v.first() {
        None => panic!("No component value to parse"),
        Some(v) => v,
    };

    lazy_static! {
        static ref VAL: Regex = Regex::new(r"^([0-9.,]*)([GMkKRmunp]?)([0-9.,]*)").unwrap();
    }

    match VAL.captures(value) {
        None => panic!("Fail to parse component value"),
        Some(cc) => {
            let left = cc.get(1).map_or("", |m| m.as_str());
            let mult = match cc.get(2).map_or("", |m| m.as_str()) {
                "G" => 12,
                "M" => 6,
                "k" | "K" => 3,
                "R" | "" => 1,
                "m" => -3,
                "u" => -6,
                "n" => -9,
                "p" => -12,
                _ => panic!("Invalid number"),
            };
            let right = cc.get(3).map_or("", |m| m.as_str());

            let left = left.replace(',', ".");
            let right = if right.is_empty() { "0" } else { right };

            let mut together = format!("{}.{}", left, right);
            if left.contains('.') {
                together = format!("{}{}", left, right);
            }

            let base = match together.parse::<f32>() {
                Err(error) => panic!(
                    "Invalid base number for convertion from string value to float {:?}",
                    error
                ),
                Ok(v) => v,
            };

            (base, mult)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_convert_comment_to_value() {
        let data = [
            ("100nF", 100e-9),
            ("1R0", 1.0),
            ("100nF", 100e-9),
            ("1R0", 1.0),
            ("1k", 1e3),
            ("2k3", 2300.0),
            ("4mH", 4e-3),
            ("12MHZ", 12e6),
            ("33nohm", 33e-9),
            ("100pF", 100e-12),
            ("1.1R", 1.1),
            ("32.768kHz", 32768.0),
            ("12.134kHz", 12134.0),
            ("100uH", 100e-6),
            ("5K421", 5421.0),
            ("2.2uH", 2.2e-6),
            ("0.3", 0.3),
            ("4.7mH inductor", 4.7e-3),
            ("0.33R", 0.33),
            ("1R1", 1.1),
            ("1R2", 1.2),
            ("0R3", 0.3),
            ("1R8", 1.8),
            ("1.1R", 1.1),
            ("1.2R", 1.2),
            ("0.3R", 0.3),
            ("1.8R", 1.8),
            ("1k5", 1500.0),
            ("1", 1.0),
            ("10R", 10.0),
            ("0.1uF", 0.1e-6),
            ("1F", 1.0),
            ("10pF", 10e-12),
            ("47uF", 47e-6),
            ("1uF", 1e-6),
            ("1nH", 1e-9),
            ("1H", 1.0),
            ("10pH", 10e-12),
            ("47uH", 47e-6),
            ("68ohm", 68.0),
            ("3.33R", 3.33),
            ("0.12R", 0.12),
            ("1.234R", 1.234),
            ("0.33R", 0.33),
            ("1MHz", 1e6),
            ("100uH", 100e-6),
            ("2k31", 2310.0),
            ("10k12", 10120.0),
            ("5K421", 5421.0),
            ("4R123", 4.123),
            ("1M12", 1.12e6),
            ("NP", -1.0),
            ("NP (0R)", -1.0),
            ("NP (AABBCC)", -1.0),
        ];

        for i in data.iter() {
            let a = convert_comment_to_value(i.0);
            println!("({:.3}, {:3}, \"{}\"),", a.0, a.1, i.0);
        }
    }

    #[test]
    fn test_value_to_eng_notation() {
        let data = [
            (100.000, -9, "F", "100nF"),
            (1.000, 1, "ohm", "1R"),
            (100.000, -9, "F", "100nF"),
            (1.000, 3, "ohm", "1k"),
            (2.300, 3, "ohm", "2k3"),
            (4.000, -3, "H", "4mH"),
            (12.000, 6, "Hz", "12MHz"),
            (33.000, -9, "ohm", "33nohm"),
            (100.000, -12, "F", "100pF"),
            (1.100, 1, "ohm", "1.1R"),
            (32.768, 3, "Hz", "32.768kHz"),
            (12.134, 3, "Hz", "12.134kHz"),
            (100.000, -6, "H", "100uH"),
            (2.200, -6, "F", "2.2uF"),
            (0.300, 1, "ohm", "0.3R"),
            (4.700, -3, "H", "4.7mH"),
            (0.330, 1, "ohm", "0.33R"),
            (1.100, 1, "ohm", "1.1R"),
            (1.200, 1, "ohm", "1.2R"),
            (0.300, 1, "ohm", "0.3R"),
            (1.800, 1, "ohm", "1.8R"),
            (1.100, 1, "ohm", "1.1R"),
            (1.200, 1, "ohm", "1.2R"),
            (0.300, 1, "ohm", "0.3R"),
            (1.500, 3, "ohm", "1k5"),
            (1.000, 1, "ohm", "1R"),
            (10.000, 1, "ohm", "10R"),
            (0.100, -6, "F", "0.1uF"),
            (1.000, 1, "F", "1F"),
            (10.000, -12, "F", "10pF"),
            (47.000, -6, "F", "47uF"),
            (1.000, -6, "F", "1uF"),
            (1.000, -9, "H", "1nH"),
            (1.000, 1, "H", "1H"),
            (10.000, -12, "H", "10pH"),
            (47.000, -6, "H", "47uH"),
            (68.000, 1, "ohm", "68R"),
            (3.330, 1, "ohm", "3.33R"),
            (0.120, 1, "ohm", "0.12R"),
            (1.234, 1, "ohm", "1.234R"),
            (0.330, 1, "ohm", "0.33R"),
            (1.000, 6, "Hz", "1MHz"),
            (100.000, -6, "H", "100uH"),
            (2.310, 3, "ohm", "2k31"),
            (10.120, 3, "ohm", "10k12"),
            (5.421, 3, "ohm", "5k421"),
            (4.123, 1, "ohm", "4.123R"),
            (1.120, 6, "ohm", "1M12"),
            (-1.0, 0, "", "NP"),
        ];

        for i in data.iter() {
            assert_eq!(value_to_eng_notation(i.0, i.1, i.2), i.3);
        }
        //assert_eq!(0, 1);
    }
    #[test]
    fn test_detect_measure_unit() {
        let test_data = [
            ["C123", "F"],
            ["R123", "ohm"],
            ["L232", "H"],
            ["Y123", "Hz"],
            ["Q123", "unknow"],
            ["TR123", "unknow"],
        ];

        for data in test_data.iter() {
            assert_eq!(detect_measure_unit(data[0]), data[1]);
        }
    }
}
