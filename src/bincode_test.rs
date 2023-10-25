use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
enum test_enum{
    AA{a: u64, b: String},
    BB{text: String}
}



fn main(){
    let a = test_enum::AA { a: 12319, b: "BEBRA".into() };
    let encoded = bincode::serialize(&a).unwrap();
    
    let decoded: test_enum = bincode::deserialize(&encoded).unwrap();
    match decoded{
        test_enum::AA { a, b } => {println!("{:?}", (a, b));},
        _ => {}
    }
    //


}