
use std::process::Command;
use std::io::Write;

const EXPECTED_OUTPUT: &str = r"
client, available, held, total, locked
1, 75.0000, 0.0000, 75.0000, true
2, 200.0000, 0.0000, 200.0000, false
3, 75.0000, 0.0000, 75.0000, false

";

const INPUT: &str = r"
type, client, tx, amount
deposit, 1, 1, 100.0
deposit, 2, 2, 200.0
deposit, 1, 3, 50.0
withdrawal, 1, 4, 25.0
dispute, 1, 1, 
resolve, 1, 1, 
deposit, 3, 5, 75.0
dispute, 3, 100, 
resolve, 3, 100, 
chargeback, 3, 100, 
dispute, 1, 3, 
chargeback, 1, 3, 

";

fn normalize_csv(content: &str) -> Vec<String> {
    content.lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .map(|line| line.to_string())
        .collect()
}

#[test]
fn test_transaction_engine_binary() {
    // Get the path to the binary using the CARGO_BIN_EXE environment variable
    let bin_path = env!("CARGO_BIN_EXE_transaction_engine");
    
    // Create a temporary input file
    let mut temp_file = tempfile::NamedTempFile::new()
        .expect("Failed to create temporary file");
    
    temp_file.write_all(INPUT.as_bytes())
        .expect("Failed to write to temporary file");
    
    let input_path = temp_file.path();
    
    // Run the binary with the test input file
    let output = Command::new(bin_path)
        .arg(input_path)
        .output()
        .expect("Failed to execute binary");
    
    assert!(output.status.success(), 
        "Binary failed with stderr: {}", 
        String::from_utf8_lossy(&output.stderr));
    
    // Get the actual output
    let actual_output = String::from_utf8_lossy(&output.stdout);
    
    // Normalize both outputs (trim whitespace, normalize line endings)
    let mut actual_lines = normalize_csv(&actual_output);
    let mut expected_lines = normalize_csv(EXPECTED_OUTPUT);

    actual_lines.sort();
    expected_lines.sort();
    
    // Compare the outputs
    assert_eq!(
        actual_lines.len(),
        expected_lines.len(),
        "Output has different number of lines.\nExpected:\n{}\n\nActual:\n{}",
        EXPECTED_OUTPUT,
        actual_output
    );
    
    for (i, (actual, expected)) in actual_lines.iter().zip(expected_lines.iter()).enumerate() {
        assert_eq!(
            actual,
            expected,
            "Line {} differs.\nExpected: {}\nActual: {}",
            i + 1,
            expected,
            actual
        );
    }
    
    println!("Test passed! Output matches expected output.");
}