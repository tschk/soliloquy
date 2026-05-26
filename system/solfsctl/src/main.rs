use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("solfsctl: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> solfsctl::Result<()> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("mkfs") => {
            let source = args
                .next()
                .ok_or_else(|| solfsctl::SolfsError::Invalid("missing source directory".into()))?;
            let output = args
                .next()
                .ok_or_else(|| solfsctl::SolfsError::Invalid("missing output image".into()))?;
            let image = solfsctl::build_image(source, output)?;
            println!(
                "solfs image entries={} size={}",
                image.header.entry_count, image.header.image_size
            );
        }
        Some("inspect") => {
            let image = args
                .next()
                .ok_or_else(|| solfsctl::SolfsError::Invalid("missing image path".into()))?;
            let image = solfsctl::inspect_image(image)?;
            println!("{}", solfsctl::render_text(&image));
        }
        Some(command) => {
            return Err(solfsctl::SolfsError::Invalid(format!(
                "unknown command: {command}"
            )));
        }
        None => {
            return Err(solfsctl::SolfsError::Invalid(
                "usage: solfsctl mkfs <source-dir> <image> | solfsctl inspect <image>".into(),
            ));
        }
    }
    Ok(())
}
