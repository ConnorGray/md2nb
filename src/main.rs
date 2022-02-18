mod ast;
mod kernel;
mod nb;


use std::path::PathBuf;

use clap::Parser;

use wolfram_app_discovery::WolframApp;
use wolfram_expr::{Expr, Symbol};

use crate::kernel::KernelProcess;

/// Discovery local installations of the Wolfram Language and Wolfram products.
#[derive(Parser, Debug)]
struct Args {
    /// Markdown input file.
    input: PathBuf,

    /// Output file location. (default: <INPUT>.nb)
    ///
    /// If this is a directory, the output notebook file will have the same file name
    /// as the input file.
    output: Option<PathBuf>,
}

fn main() -> Result<(), kernel::Error> {
    let Args { input, output } = Args::parse();

    let contents: String =
        std::fs::read_to_string(&input).expect("failed to read input file");

    let ast = ast::parse_markdown_to_ast(&contents);

    /* For debugging.
    println!("\n\n===== AST =====\n");
    for block in &ast {
        println!("block: {block:?}\n");
    }
    println!("\n\n===== End AST =====\n");
    */

    //----------------------------------------------------------------
    // Determine the output file location
    //----------------------------------------------------------------

    // If `output` is a directory, automatically determine the file name from `input`.
    // E.g. `$ md2nb README.md` will automatically write to `./README.nb`.
    let auto_file_name = format!("{}.nb", input.file_stem().unwrap().to_str().unwrap());

    let output = match output {
        Some(output) if output.is_dir() => output.join(auto_file_name),
        Some(output) => output,
        None => std::env::current_dir().unwrap().join(auto_file_name),
    };

    // TODO: This has a TOCTOU race. `output` may not exist now, but another program
    //       could create it before we do. Considering the startup time of the Kernel
    //       and the time it takes to generate larger files, that span will often be
    //       several seconds at least.
    // TODO: Support an `--overwrite` or `-f, --force` option to disable this.
    //       NotebookSave will overwrite by default.
    if output.exists() {
        panic!("error: output file already exists: {}", output.display())
    }

    //----------------------------------------------------------------
    // Convert the Markdown AST to a sequence of Cell[..] expressions.
    //----------------------------------------------------------------

    let cells: Vec<Expr> = ast.into_iter().flat_map(nb::block_to_cells).collect();

    //----------------------------------------------------------
    // Launch the Kernel, and write the cells to a new notebook.
    //----------------------------------------------------------

    let mut kernel = launch_default_kernel()?;

    let nb_obj = create_notebook(&mut kernel)?;

    for cell in cells {
        // NotebookWrite[nb_obj, cell]
        kernel
            .put_eval_packet(&using_front_end(Expr::normal(
                Symbol::new("System`NotebookWrite"),
                vec![nb_obj.clone(), cell],
            )))
            .unwrap();
    }

    // NotebookSave[nb_obj, output]
    kernel
        .put_eval_packet(&using_front_end(Expr::normal(
            Symbol::new("System`NotebookSave"),
            vec![
                nb_obj,
                Expr::from(
                    output
                        .to_str()
                        .expect("output file path cannot be converted to a &str"),
                ),
            ],
        )))
        .unwrap();

    drop(kernel);

    unsafe {
        // Shut the WSTP library down gracefully.
        wstp::shutdown()?;
    }

    Ok(())
}

fn using_front_end(expr: Expr) -> Expr {
    Expr::normal(Symbol::new("System`UsingFrontEnd"), vec![expr])
}

fn create_notebook(kernel: &mut KernelProcess) -> Result<Expr, kernel::Error> {
    let () = kernel.put_eval_packet(&using_front_end(Expr::normal(
        Symbol::new("System`CreateNotebook"),
        vec![],
    )))?;

    skip_to_next_return_packet(kernel.link())?;

    Ok(get_system_expr(kernel.link())?)
}

fn launch_default_kernel() -> Result<KernelProcess, kernel::Error> {
    let app = WolframApp::try_default()
        .expect("unable to find any Wolfram Language installations");

    let kernel = app.kernel_executable_path().unwrap();

    KernelProcess::launch(&kernel)
}

fn skip_to_next_return_packet(link: &mut wstp::Link) -> Result<(), wstp::Error> {
    use wstp::sys::*;

    loop {
        match link.raw_next_packet()? {
            RETURNPKT => break,
            _pkt => {
                // println!("\npacket: {pkt}");
                // dump_tokens(link, 0).unwrap();
                let () = link.new_packet()?;
                continue;
            },
        }
    }

    Ok(())
}

fn get_system_expr(link: &mut wstp::Link) -> Result<Expr, wstp::Error> {
    link.get_expr_with_resolver(&mut |sym: &str| {
        let abs = format!("System`{sym}");
        Some(Symbol::try_new(&abs).expect("unexpected WSTP symbol syntax"))
    })
}

/// Read all remaining data on the link and debug print it.
#[allow(dead_code)]
fn dump_tokens(link: &mut wstp::Link, indent: usize) -> Result<(), wstp::Error> {
    use wstp::Token;

    let pad = format!("{:indent$}", "");

    let value = link.get()?;

    match value {
        Token::Integer(value) => println!("token: {pad}{value}"),
        Token::Real(value) => println!("token: {pad}{value}"),
        Token::String(value) => println!("token: {pad}{}", value.as_str()),
        Token::Symbol(value) => println!("token: {pad}{}", value.as_str()),
        Token::Normal(length) => {
            drop(value);

            dump_tokens(link, indent)?;

            for _ in 0..length {
                dump_tokens(link, indent + 4)?
            }
        },
    }

    Ok(())
}
