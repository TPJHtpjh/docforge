use std::fs;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path_str = args
        .get(1)
        .map(|s| s.as_str())
        .unwrap_or("./tests/期末作业要求.docx");
    let path = std::path::Path::new(path_str);
    let bytes = fs::read(path).expect("failed to read docx");

    let docx_json = docx_rs::read_docx(&bytes).expect("failed to parse docx");
    let json_str = docx_json.json();

    // Pretty print
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    let pretty = serde_json::to_string_pretty(&parsed).unwrap();

    fs::write("./tests/debug_docx_output.json", &pretty).unwrap();
    println!("JSON written to tests/debug_docx_output.json");
    println!("--- First 3000 chars ---");
    println!("{}", &pretty[..pretty.len().min(3000)]);
}
