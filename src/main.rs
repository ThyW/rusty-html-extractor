use std::io::Write;

use hp::{Parser, Template};
use html2text::{from_read_with_decorator, render::text_renderer::TrivialDecorator};
use zip::read::ZipArchive;

const DEFAULT_OUTPUT_TEXT: &str = "./html_text.txt";
const DEFAULT_OUTPUT_REST: &str = "./rest";

#[derive(Debug)]
enum MyError {
    Io(std::io::Error),
    Msg(&'static str),
}

impl From<zip::result::ZipError> for MyError {
    fn from(_: zip::result::ZipError) -> Self {
        Self::Msg("Zip error occured")
    }
}

impl From<&'static str> for MyError {
    fn from(value: &'static str) -> Self {
        Self::Msg(value)
    }
}

impl From<std::io::Error> for MyError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl std::fmt::Display for MyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "{}", e),
            Self::Msg(e) => write!(f, "{}", e),
        }
    }
}

fn parse(input: impl AsRef<str>, output: &[impl AsRef<str>]) -> Result<(), MyError> {
    if !std::path::PathBuf::from(input.as_ref()).exists() {
        return Err("Input file does not exist".into());
    }
    let file = std::fs::File::open(input.as_ref())?;
    let mut archive = ZipArchive::new(file)?;
    let mut output_text = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(output[0].as_ref())?;
    std::fs::create_dir_all(output[1].as_ref())?;
    let output_dir = std::path::PathBuf::from(output[1].as_ref());
    for i in 0..archive.len() {
        let mut archive_file = archive.by_index(i)?;
        let fname = match archive_file.enclosed_name() {
            Some(p) => p,
            None => continue,
        };

        if fname.extension().is_some() && fname.extension().unwrap().to_str().unwrap() == "html" {
            let html_text = from_read_with_decorator(archive_file, 80, TrivialDecorator::new());
            output_text.write_all(html_text.as_bytes())?;
        } else if fname.to_str().unwrap().ends_with('/') {
            let new_path = output_dir.clone().join(fname);
            std::fs::create_dir_all(new_path)?;
        } else {
            let mut outfile = std::fs::File::create(output_dir.clone().join(fname))?;
            std::io::copy(&mut archive_file, &mut outfile)?;
        }
    }
    Ok(())
}

fn main() -> std::io::Result<()> {
    let mut parser = Parser::new()
        .with_description("Extract text from zipped html files.")
        .exit_on_help(true);
    let input = parser.add_template(
        Template::new()
            .matches("-i")
            .matches("--input")
            .with_help("Input file.")
            .optional_values(false)
            .number_of_values(1),
    );
    let output = parser.add_template(
        Template::new()
            .matches("-o")
            .matches("--output")
            .with_help("Two output files. First is the path to the extracted html, second is the directory where the rest of the files will be stored. Defaults are `html_text.txt` and `rest/`")
            .number_of_values(2)
            .optional_values(false),
    );

    if let Ok(pargs) = parser.parse(None) {
        if pargs.has_with_id(input) {
            let mut output_files = vec![
                DEFAULT_OUTPUT_TEXT.to_string(),
                DEFAULT_OUTPUT_REST.to_string(),
            ];
            if pargs.has_with_id(output) {
                output_files = pargs.get_with_id(output).unwrap().values().clone();
            }
            if let Err(error) = parse(
                &pargs.get_with_id(input).unwrap().values()[0],
                &output_files,
            ) {
                println!("ERROR OCCURED: {}", error);
            }
        }
    }
    Ok(())
}
