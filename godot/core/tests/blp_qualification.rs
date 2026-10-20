//! Before-only observations; never changes or reconciles decoder outcomes.
use game_engine_core::blp;
use serde_json::{Value, json};
use std::any::Any;
use std::fs;
use std::panic::{UnwindSafe, catch_unwind};
use std::path::Path;

fn panic_text(payload: &dyn Any) -> &str {
    if let Some(message) = payload.downcast_ref::<String>() {
        message
    } else if let Some(message) = payload.downcast_ref::<&str>() {
        message
    } else {
        "non-string panic"
    }
}

fn capture(
    output: &Path,
    name: &str,
    decode: impl FnOnce() -> Result<(Value, Vec<u8>), String> + UnwindSafe,
) -> Value {
    match catch_unwind(decode) {
        Ok(Ok((mut metadata, bytes))) => {
            metadata["status"] = json!("ok");
            metadata["bytes"] = json!(bytes.len());
            fs::write(output.join(format!("{name}.bin")), bytes).unwrap();
            metadata
        }
        Ok(Err(error)) => json!({"status": "error", "message": error}),
        Err(payload) => json!({"status": "panic", "message": panic_text(payload.as_ref())}),
    }
}

#[test]
#[ignore = "qualification needs explicit frozen input manifest and new receipt directory"]
fn capture_cpu_gpu_before_only() {
    let input = std::env::var("WOW_BLP_INPUTS").expect("WOW_BLP_INPUTS");
    let output = std::env::var("WOW_BLP_ENGINE_OUTPUT").expect("WOW_BLP_ENGINE_OUTPUT");
    let output = Path::new(&output);
    fs::create_dir(output).expect("receipt directory must be new");
    let manifest: Value = serde_json::from_slice(&fs::read(input).unwrap()).unwrap();
    let cases = manifest["cases"].as_array().unwrap();
    assert!(cases.iter().any(|case| case["kind"] == "real"));
    assert!(cases.iter().any(|case| case["kind"] == "synthetic"));
    let mut receipts = Vec::new();
    for case in cases {
        let name = case["name"].as_str().unwrap();
        let bytes = fs::read(case["path"].as_str().unwrap()).unwrap();
        assert_eq!(bytes.len() as u64, case["bytes"].as_u64().unwrap());
        let cpu = capture(output, &format!("{name}.cpu"), || {
            blp::decode_rgba(&bytes).map(|image| {
                (
                    json!({"width": image.width, "height": image.height}),
                    image.pixels,
                )
            })
        });
        let gpu = capture(output, &format!("{name}.gpu"), || {
            blp::decode_gpu(&bytes).map(|image| {
                (
                    json!({"width": image.width, "height": image.height,
                        "format": format!("{:?}", image.format), "mipmaps": image.mipmaps}),
                    image.data,
                )
            })
        });
        receipts.push(json!({"name": name, "cpu": cpu, "gpu": gpu}));
    }
    assert_eq!(receipts.len(), cases.len());
    fs::write(
        output.join("outcomes.json"),
        serde_json::to_vec_pretty(&receipts).unwrap(),
    )
    .unwrap();
    println!("Captured {} independent CPU/GPU outcomes", receipts.len());
}
