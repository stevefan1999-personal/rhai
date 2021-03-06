use rhai::{Dynamic, Engine, EvalAltResult, Scope, AST};

#[cfg(not(feature = "no_optimize"))]
use rhai::OptimizationLevel;

use std::io::{stdin, stdout, Write};

fn print_error(input: &str, err: EvalAltResult) {
    let lines: Vec<_> = input.trim().split('\n').collect();
    let pos = err.position();

    let line_no = if lines.len() > 1 {
        if pos.is_none() {
            "".to_string()
        } else {
            format!("{}: ", pos.line().unwrap())
        }
    } else {
        "".to_string()
    };

    // Print error
    let pos_text = format!(" ({})", pos);

    if pos.is_none() {
        // No position
        println!("{}", err);
    } else {
        // Specific position
        println!("{}{}", line_no, lines[pos.line().unwrap() - 1]);

        let err_text = match err {
            EvalAltResult::ErrorRuntime(err, _) if !err.is_empty() => {
                format!("Runtime error: {}", err)
            }
            err => err.to_string(),
        };

        println!(
            "{0:>1$} {2}",
            "^",
            line_no.len() + pos.position().unwrap(),
            err_text.replace(&pos_text, "")
        );
    }
}

fn print_help() {
    println!("help       => print this help");
    println!("quit, exit => quit");
    println!("ast        => print the last AST");
    println!("astu       => print the last raw, un-optimized AST");
    println!(r"end a line with '\' to continue to the next line.");
    println!();
}

fn main() {
    let mut engine = Engine::new();

    #[cfg(not(feature = "no_optimize"))]
    engine.set_optimization_level(OptimizationLevel::None);

    let mut scope = Scope::new();

    let mut input = String::new();
    let mut main_ast: AST = Default::default();
    let mut ast_u: AST = Default::default();
    let mut ast: AST = Default::default();

    println!("Rhai REPL tool");
    println!("==============");
    print_help();

    loop {
        print!("rhai> ");
        stdout().flush().expect("couldn't flush stdout");

        input.clear();

        loop {
            if let Err(err) = stdin().read_line(&mut input) {
                panic!("input error: {}", err);
            }

            let line = input.as_str().trim_end();

            // Allow line continuation
            if line.ends_with('\\') {
                let len = line.len();
                input.truncate(len - 1);
                input.push('\n');
            } else {
                break;
            }

            print!("> ");
            stdout().flush().expect("couldn't flush stdout");
        }

        let script = input.trim();

        if script.is_empty() {
            continue;
        }

        // Implement standard commands
        match script {
            "help" => {
                print_help();
                continue;
            }
            "exit" | "quit" => break, // quit
            "astu" => {
                // print the last un-optimized AST
                println!("{:#?}", &ast_u);
                continue;
            }
            "ast" => {
                // print the last AST
                println!("{:#?}", &ast);
                continue;
            }
            _ => (),
        }

        match engine
            .compile_with_scope(&scope, &script)
            .map_err(Into::into)
            .and_then(|r| {
                ast_u = r.clone();

                #[cfg(not(feature = "no_optimize"))]
                {
                    ast = engine.optimize_ast(&scope, r, OptimizationLevel::Full);
                }

                #[cfg(feature = "no_optimize")]
                {
                    ast = r;
                }

                // Merge the AST into the main
                main_ast = main_ast.merge(&ast);

                // Evaluate
                engine.eval_ast_with_scope::<Dynamic>(&mut scope, &main_ast)
            }) {
            Ok(result) if !result.is::<()>() => {
                println!("=> {:?}", result);
                println!();
            }
            Ok(_) => (),
            Err(err) => {
                println!();
                print_error(&input, *err);
                println!();
            }
        }

        // Throw away all the statements, leaving only the functions
        #[cfg(not(feature = "no_function"))]
        main_ast.retain_functions();
    }
}
