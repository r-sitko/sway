use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use sway_core::{compile_to_ast, ir_generation::compile_program, namespace, CompileAstResult};

pub(super) fn run(filter_regex: Option<&regex::Regex>) {
    // Compile core library and reuse it when compiling tests.
    let core_lib = compile_core();

    // Find all the tests.
    discover_test_files()
        .into_iter()
        .filter(|path| {
            // Filter against the regex.
            path.to_str()
                .and_then(|path_str| filter_regex.map(|regex| regex.is_match(path_str)))
                .unwrap_or(true)
        })
        .map(|path| {
            // Read entire file.
            let input_bytes = fs::read(&path).expect("Read entire Sway source.");
            let input = String::from_utf8_lossy(&input_bytes);

            // Split into Sway, FileCheck of IR, FileCheck of ASM.
            //
            // - Search for the optional boundaries.  If they exist, delimited by special tags,
            // then they mark the boundaries for their checks.  If the IR delimiter is missing then
            // it's assumed to be from the start of the file.  The ASM checks themselves are
            // entirely optional.
            let ir_checks_begin_offs = input.find("::check-ir::").unwrap_or(0);
            let asm_checks_begin_offs = input.find("::check-asm::");

            let ir_checks_end_offs = match asm_checks_begin_offs {
                Some(asm_offs) if asm_offs > ir_checks_begin_offs => asm_offs,
                _otherwise => input.len(),
            };

            let ir_checker = filecheck::CheckerBuilder::new()
                .text(&input[ir_checks_begin_offs..ir_checks_end_offs])
                .unwrap()
                .finish();

            let asm_checker = asm_checks_begin_offs.map(|begin_offs| {
                let end_offs = if ir_checks_begin_offs > begin_offs {
                    ir_checks_begin_offs
                } else {
                    input.len()
                };
                filecheck::CheckerBuilder::new()
                    .text(&input[begin_offs..end_offs])
                    .unwrap()
                    .finish();
            });

            (path, input_bytes, ir_checker, asm_checker)
        })
        .for_each(|(path, sway_str, ir_checker, opt_asm_checker)| {
            let sway_str = String::from_utf8_lossy(&sway_str);

            println!("IR CHECKING {}.", path.display());

            // Compile to AST.
            let typed_program = match compile_to_ast(Arc::from(sway_str), core_lib.clone(), None) {
                CompileAstResult::Success { typed_program, .. } => typed_program,
                CompileAstResult::Failure { errors, .. } => panic!(
                    "Failed to compile test {}:\n{}",
                    path.display(),
                    errors
                        .iter()
                        .map(|err| err.to_string())
                        .collect::<Vec<_>>()
                        .as_slice()
                        .join("\n")
                ),
            };

            // Compile to IR.
            let ir = compile_program(*typed_program).unwrap();
            let ir_output = sway_ir::printer::to_string(&ir);

            if ir_checker.is_empty() {
                panic!(
                    "IR test for {} is missing mandatory FileCheck directives.\n\n\
                    Here's the IR output:\n{ir_output}",
                    path.file_name().unwrap().to_string_lossy()
                );
            }

            // Do IR checks.
            match ir_checker.explain(&ir_output, filecheck::NO_VARIABLES) {
                Ok((success, report)) if !success => {
                    panic!("IR filecheck failed:\n{report}");
                }
                Err(e) => {
                    panic!("IR filecheck directive error: {e}");
                }
                _ => (),
            };

            // Do ASM checks.
            if let Some(_asm_checker) = opt_asm_checker {
                // Compile to ASM.
                // The tests are in from_ir.rs, pretty similar.  You can .to_string() the asm.
                //match asm_checker.explain(&ir_output, filecheck::NO_VARIABLES) {
                //    Ok((success, report)) if !success => {
                //        panic!("IR filecheck failed:\n{report}");
                //    }
                //    Err(e) => {
                //        panic!("IR filecheck directive error: {e}");
                //    }
                //    _ => (),
                //};
            }

            // Test the IR parser.
            println!("PASSED.");
        });
}

fn discover_test_files() -> Vec<PathBuf> {
    fn recursive_search(path: &Path, test_files: &mut Vec<PathBuf>) {
        if path.is_dir() {
            for entry in fs::read_dir(path).unwrap() {
                recursive_search(&entry.unwrap().path(), test_files);
            }
        } else if path.is_file() && path.extension().map(|ext| ext == "sw").unwrap_or(false) {
            test_files.push(path.to_path_buf());
        }
    }

    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let tests_root_dir = format!("{manifest_dir}/src/ir_generation/tests");

    let mut test_files = Vec::new();
    recursive_search(&PathBuf::from(tests_root_dir), &mut test_files);
    test_files
}

fn compile_core() -> namespace::Module {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let libcore_root_dir = format!("{manifest_dir}/../sway-lib-core");

    let check_cmd = forc::cli::CheckCommand {
        path: Some(libcore_root_dir),
        offline_mode: true,
        silent_mode: true,
        locked: false,
    };

    match forc::test::forc_check::check(check_cmd)
        .expect("Failed to compile sway-lib-core for IR tests.")
    {
        CompileAstResult::Success { typed_program, .. } => typed_program.root.namespace,
        _ => panic!("Failed to compile sway-lib-core for IR tests."),
    }
}

//|
//|// Load each Sway source file found, compile it to IR and use FileCheck to verify.
//|fn sway_to_ir_tests() {
//|    let manifest_dir = env!("CARGO_MANIFEST_DIR");
//|    let dir: PathBuf = format!("{}/src/tests", manifest_dir).into();
//|    for entry in std::fs::read_dir(dir).unwrap() {
//|        // We're only interested in the `.sw` files here.
//|        let path = entry.unwrap().path();
//|        match path.extension().unwrap().to_str() {
//|            Some("sw") => {
//|                //
//|                // Run the tests!
//|                //
//|                tracing::info!("---- Sway To IR: {:?} ----", path);
//|                test_sway_to_ir(path);
//|            }
//|            Some("ir") | Some("disabled") => (),
//|            _ => panic!(
//|                "File with invalid extension in tests dir: {:?}",
//|                path.file_name().unwrap_or(path.as_os_str())
//|            ),
//|        }
//|    }
//|}
//|
//|fn test_sway_to_ir(sw_path: PathBuf) {
//|    let input_bytes = std::fs::read(&sw_path).unwrap();
//|    let input = String::from_utf8_lossy(&input_bytes);
//|
//|    let mut ir_path = sw_path.clone();
//|    ir_path.set_extension("ir");
//|
//|    let expected_bytes = std::fs::read(&ir_path).unwrap();
//|    let expected = String::from_utf8_lossy(&expected_bytes);
//|
//|    let typed_program = parse_to_typed_program(sw_path.clone(), &input);
//|    let ir = sway_core::compile_program(typed_program).unwrap();
//|    let output = sway_ir::printer::to_string(&ir);
//|
//|    // Use a tricky regex to replace the local path in the metadata with something generic.  It
//|    // should convert, e.g.,
//|    //     `!0 = filepath "/usr/home/me/sway/sway-core/tests/sway_to_ir/foo.sw"`
//|    //  to `!0 = filepath "/path/to/foo.sw"`
//|    let path_converter = regex::Regex::new(r#"(!\d = filepath ")(?:[^/]*/)*(.+)"#).unwrap();
//|    let output = path_converter.replace_all(output.as_str(), "$1/path/to/$2");
//|
//|    if output != expected {
//|        println!("{}", prettydiff::diff_lines(&expected, &output));
//|        panic!("{} failed.", sw_path.display());
//|    }
//|}
//|
//|fn ir_printer_parser_tests() {
//|    let manifest_dir = env!("CARGO_MANIFEST_DIR");
//|    let dir: PathBuf = format!("{}/tests/sway_to_ir", manifest_dir).into();
//|    for entry in std::fs::read_dir(dir).unwrap() {
//|        // We're only interested in the `.ir` files here.
//|        let path = entry.unwrap().path();
//|        match path.extension().unwrap().to_str() {
//|            Some("ir") => {
//|                //
//|                // Run the tests!
//|                //
//|                tracing::info!("---- IR Print and Parse Test: {:?} ----", path);
//|                test_printer_parser(path);
//|            }
//|            Some("sw") | Some("disabled") => (),
//|            _ => panic!(
//|                "File with invalid extension in tests dir: {:?}",
//|                path.file_name().unwrap_or(path.as_os_str())
//|            ),
//|        }
//|    }
//|}
//|
//|fn test_printer_parser(path: PathBuf) {
//|    let input_bytes = std::fs::read(&path).unwrap();
//|    let input = String::from_utf8_lossy(&input_bytes);
//|
//|    // Use another tricky regex to inject the proper metadata filepath back, so we can create
//|    // spans in the parser.  NOTE, if/when we refactor spans to not have the source string and
//|    // just the path these tests should pass without needing this conversion.
//|    let mut true_path = path.clone();
//|    true_path.set_extension("sw");
//|    let path_converter = regex::Regex::new(r#"(!\d = filepath )(?:.+)"#).unwrap();
//|    let input = path_converter.replace_all(&input, format!("$1\"{}\"", true_path.display()));
//|
//|    let parsed_ctx = match sway_ir::parser::parse(&input) {
//|        Ok(p) => p,
//|        Err(e) => {
//|            println!("{}: {}", path.display(), e);
//|            panic!();
//|        }
//|    };
//|    let printed = sway_ir::printer::to_string(&parsed_ctx);
//|    if printed != input {
//|        println!("{}", prettydiff::diff_lines(&input, &printed));
//|        panic!("{} failed.", path.display());
//|    }
//|}
