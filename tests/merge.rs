use mergebom_web::bom::Bom;
use std::fs::File;
use std::io::{BufRead, BufReader};

const TEST_DIR: &str = "tests/data";
const CHECK_DIR: &str = "tests/data";

fn dump(v: &[String]) {
    println!("\n---- {} -------------\n", v.len());
    for i in v.iter() {
        println!("> {}", i);
    }
    println!("\n=====================\n");
}

fn test_run(test: &str, check: &str, merge_keys: &[String]) {
    let t = format!("{}/{}", TEST_DIR, test);
    let c = format!("{}/{}", CHECK_DIR, check);

    let mut checks = vec![];
    let file = File::open(c).unwrap();
    for line in BufReader::new(file).lines().map_while(Result::ok) {
        if !line.is_empty() {
            checks.push(line);
        }
    }

    let mut results = vec![];
    let bom = Bom::loader(&[t], merge_keys);
    let data = bom.merge().odered_vector_table();
    for c in data.rows.iter() {
        results.push(format!("{:}", c));
    }
    results.push(data.headers.join(";"));

    results.sort();
    checks.sort();
    dump(&results);
    assert_eq!(results.len(), checks.len());
    for (i, r) in results.iter().enumerate() {
        println!("RESULT < - >  CHECK");
        println!("{} {}", i, r);
        assert_eq!(*r, checks[i]);
    }
}

#[test]
fn merge_tests() {
    test_run("test0.csv", "test0.check", &[]);
    test_run("test1.csv", "test1.check", &["comment"].map(String::from));
}

// #[test]
// fn connector() {
//     test_run(
//         "test2.csv",
//         "test4.check",
//         &["comment", "footprint", "description"].map(String::from),
//     );
// }

// #[test]
// fn diode() {
//     test_run(
//         "test3.csv",
//         "test5.check",
//         &["comment", "footprint", "description"].map(String::from),
//     );
// }
