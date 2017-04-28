use std::env;
use std::process::Command;

fn find_f64(source: &str, prefix: &str, suffix: &str) -> f64 {
    let index_start = source.rfind(&prefix).unwrap() + prefix.len();
    let (_, end) = source.split_at(index_start);
    let index_stop = end.find(suffix).unwrap();
    let (result_str, everything_else) = end.split_at(index_stop);
    let result = result_str.parse().unwrap();
    assert!(everything_else.find(&prefix).is_none(), "This was the only occurrence of '{}'.", prefix);
    result
}

fn main() {
    let path_jsbin = "/tmp/jsbin-test.jsbin";
    let path_jsbin_gz = format!("{}.gz", path_jsbin);
    let path_source = "/tmp/jsbin-test.js";
    let path_source_gz = format!("{}.gz", path_source);

    let path_jsshell = env::args().nth(1).expect("Expected arg: shell name");
    println!("Using js shell: {}", path_jsshell);

    // 1. Measure size.
    let mut total_source_gz_size = 0;
    let mut total_jsbin_gz_size = 0;

    println!("*** Measuring file size");

    for source in env::args().skip(2) {
        println!("Source: {}", source);

        // 1.1. Write jsbin file to disk.
        let script = format!(
            r##"
            let source = os.file.readFile("{}");
            let syntaxParsed = syntaxParse(source);
            let buf = jsbinSerialize(source);
            os.file.writeTypedArrayToFile("{}", new Uint8Array(buf));"##,
            source,
            path_jsbin);
        let jsshell_output = Command::new(path_jsshell.clone())
            .arg("-e")
            .arg(script)
            .output()
            .expect("Expected shell output");
        assert!(jsshell_output.status.success());


        // 1.2. Run gzip to convert jsbin to .jsbin.gz
        let _ = std::fs::remove_file(path_jsbin_gz.clone());
        let gzip = Command::new("/usr/bin/gzip")
            .arg(path_jsbin)
            .spawn()
            .expect("Could not spawn gzip")
            .wait()
            .expect("Gzip failed");
        assert!(gzip.success());

        // 1.3. Get size of .jsbin.gz
        let jsbin_gz_size = std::fs::metadata(path_jsbin_gz.clone()).expect("Could not find .jsbin.gz").len();
        total_jsbin_gz_size += jsbin_gz_size;

        // 1.4. Copy source
        std::fs::copy(source, path_source).expect("Could not copy source");

        // 1.5. Run gzip to convert source to .js.gz
        let _ = std::fs::remove_file(path_source_gz.clone());
        let gzip = Command::new("/usr/bin/gzip")
            .arg(path_source)
            .spawn()
            .expect("Could not spawn gzip")
            .wait()
            .expect("Gzip failed");
        assert!(gzip.success());

        // 1.6. Get size of .js.gz
        let source_gz_size = std::fs::metadata(path_source_gz.clone()).expect("Could not find .js.gz").len();
        total_source_gz_size += source_gz_size;

        println!("{}kb => {}kb (*{:.2})", source_gz_size / 1000, jsbin_gz_size / 1000, (jsbin_gz_size as f64) / (source_gz_size as f64));
    }
    println!("Total: {}kb => {}kb (*{:.2})", total_source_gz_size / 1000, total_jsbin_gz_size / 1000, (total_jsbin_gz_size as f64) / (total_source_gz_size as f64));


    // 2. Measure performance.

    for syntax_parse in &["false", "true"] {
        for depth in &[0, 1, 1000] {
            let mut total_source_full_duration = 0.;
            let mut total_source_lazy_duration = 0.;
            let mut total_jsbin_duration = 0.;

            println!("*** Measuring performance with syntaxParse: {}, depth: {}", syntax_parse, depth);

            for _ in 0..5 { // Repeat a few times to stabilize statistics.
                for source in env::args().skip(2) {
                    println!("Source: {}", source);

                    // 1. Run jsshell to test and convert to jsbin.
                    let script = format!(
                        r##"
                        let source = os.file.readFile("{}");
                        let syntaxParsed = syntaxParse(source);
                        let buf = jsbinSerialize(source);
                        for (let i = 0; i < 1; ++i) {{
                            jsbinParse(buf, {{
                                skipContentsAfterFunctionDepth: {},
                                syntaxParse: {},
                            }}); // Check that parsing works.
                        }}"##, source, depth, syntax_parse);
                    let jsshell_output = Command::new(path_jsshell.clone())
                        .arg("-e")
                        .arg(script)
                        .output()
                        .expect("Expected shell output");
                    assert!(jsshell_output.status.success());
                    let jsshell_stderr = String::from_utf8(jsshell_output.stderr).expect("Expected correct UTF8");

                    let source_full_duration = find_f64(&jsshell_stderr, "Parser<>::parse() full duration: ", "ms");
                    total_source_full_duration += source_full_duration;
                    let source_lazy_duration = find_f64(&jsshell_stderr, "Parser<>::parse() lazy duration: ", "ms");
                    total_source_lazy_duration += source_lazy_duration;
                    let jsbin_duration = find_f64(&jsshell_stderr, "ReadBinaryAST duration: ", "ms");
                    total_jsbin_duration += jsbin_duration;

                    println!("{:.2}ms/{:.2}ms => {:.2}ms (*{:.2}/{:.2})", source_lazy_duration, source_full_duration, jsbin_duration, jsbin_duration / source_lazy_duration, jsbin_duration / source_full_duration);
                }
            }

            println!("Results with syntaxParse: {}, depth: {}", syntax_parse, depth);
            println!("Total: {:.2}ms/{:.2}ms => {:.2}ms (*{:.2}/{:.2})", total_source_lazy_duration, total_source_full_duration, total_jsbin_duration, total_jsbin_duration / total_source_lazy_duration, total_jsbin_duration / total_source_full_duration);
        }
    }
}
