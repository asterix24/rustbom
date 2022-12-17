use mergebom_web::bom::{merge_key_list, Bom, ItemsTable};

#[test]
fn merge() {
    let bom = Bom::loader("data/bom_test1.csv", &["comment".to_string()]);
    let data = bom.merge().odered_vector_table();
    println!("{:?}", data);
}
