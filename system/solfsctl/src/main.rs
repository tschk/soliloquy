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
            let first = args
                .next()
                .ok_or_else(|| solfsctl::SolfsError::Invalid("missing source directory".into()))?;
            let (mode, source) = if first == "--mutable" {
                (
                    solfsctl::ImageMode::Mutable,
                    args.next().ok_or_else(|| {
                        solfsctl::SolfsError::Invalid("missing source directory".into())
                    })?,
                )
            } else {
                (solfsctl::ImageMode::ReadOnly, first)
            };
            let output = args
                .next()
                .ok_or_else(|| solfsctl::SolfsError::Invalid("missing output image".into()))?;
            let image = solfsctl::build_image_with_mode(source, output, mode)?;
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
        Some("read") => {
            let image = args
                .next()
                .ok_or_else(|| solfsctl::SolfsError::Invalid("missing image path".into()))?;
            let path = args
                .next()
                .ok_or_else(|| solfsctl::SolfsError::Invalid("missing file path".into()))?;
            let bytes = solfsctl::read_file(image, &path)?;
            print!("{}", String::from_utf8_lossy(&bytes));
        }
        Some("write") => {
            let image = args
                .next()
                .ok_or_else(|| solfsctl::SolfsError::Invalid("missing image path".into()))?;
            let path = args
                .next()
                .ok_or_else(|| solfsctl::SolfsError::Invalid("missing file path".into()))?;
            let value = args
                .next()
                .ok_or_else(|| solfsctl::SolfsError::Invalid("missing value".into()))?;
            solfsctl::overwrite_file(image, &path, value.as_bytes())?;
        }
        Some(command) => {
            return Err(solfsctl::SolfsError::Invalid(format!(
                "unknown command: {command}"
            )));
        }
        None => {
            return Err(solfsctl::SolfsError::Invalid(
                "usage: solfsctl mkfs [--mutable] <source-dir> <image> | solfsctl inspect <image> | solfsctl read <image> <path> | solfsctl write <image> <path> <value>".into(),
            ));
        }
    }
    Ok(())
}
