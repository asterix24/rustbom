use mergebom_web::bom::Bom;
use std::fs::File;
use std::io::{BufRead, BufReader};

const TEST_DIR: &str = "tests/data";
const CHECK_DIR: &str = "tests/data";

fn test_run(test: &str, check: &str, merge_keys: &[String]) {
    let t = format!("{}/{}", TEST_DIR, test);
    let c = format!("{}/{}", CHECK_DIR, check);

    let bom = Bom::loader(&[t], merge_keys);
    let data = bom.merge().odered_vector_table();

    let file = File::open(c).unwrap();
    let reader = BufReader::new(file);
    for (index, line) in reader.lines().enumerate() {
        let line = line.unwrap();
        let m = format!("{:}", data.rows[index]);
        println!("{}. {} -> {}", index + 1, line, m);

        assert_eq!(line, m);
    }
}

#[test]
fn merge() {
    test_run("bom_test1.csv", "bom_check1.csv", &["comment".to_string()]);
}
