use std::{
    fs::File,
    io::{Cursor, Read, Seek, SeekFrom, Write},
    path::PathBuf,
};

use anyhow::Result as AResult;
use byteorder::{ReadBytesExt, WriteBytesExt, LE};
use clap::{Parser};

/// Extract/inject BBS from/into the .uexp and .uasset files of GGST/DBFZ
#[derive(Parser)]
#[clap(author = "Original 'ggst-bbs-unpacker' by Pangaea - 'bbspack' fork by Broscar")]
#[clap(arg_required_else_help(true), subcommand_required(true))]
enum Args {
    /// Extract BBS
    Extract {
        /// The .uexp to extract the BBS from
        #[clap(parse(from_os_str))]
        uexp: PathBuf,
        /// Output path+filename for the BBS
        #[clap(parse(from_os_str))]
        output: PathBuf,
    },
    /// Inject BBS
    Inject {
        /// The BBS you want to inject
        #[clap(parse(from_os_str))]
        file: PathBuf,
        /// The .uexp you want to inject the script into
        #[clap(parse(from_os_str))]
        uexp: PathBuf,
        /// The .uasset that matches the .uexp file
        #[clap(parse(from_os_str))]
        uasset: PathBuf,
    },
}

fn main() {
    if let Err(e) = run() {
        println!("ERROR: {}", e);
    }
}

fn run() -> AResult<()> {
    let args = Args::parse();

    match args {
        Args::Extract {
            uexp,
            output,
        } => extract_file(uexp, output),
        Args::Inject {
            file,
            uexp,
            uasset,
        } => inject_file(file, uexp, uasset),
    }?;

    Ok(())
}

pub fn extract_file(uexp: PathBuf, output: PathBuf) -> AResult<()> {
    let mut file = File::create(output)?;
    let mut uexp = File::open(uexp)?;
    let mut uexp_bytes = Vec::new();

    uexp.read_to_end(&mut uexp_bytes)?;

    let contained_file = &uexp_bytes[UEXP_FILE_START..uexp_bytes.len() - 0x4];

    file.write_all(contained_file)?;

    Ok(())
}

/// offset of the 2 values that both hold the size of the contained file
const UEXP_SIZE_OFFSET: usize = 0x24;
const UEXP_FILE_START: usize = 0x34;

pub fn inject_file(inject: PathBuf, uexp: PathBuf, uasset: PathBuf) -> AResult<()> {

    let uexp_path = uexp.clone();
    let uasset_path = uasset.clone();

    let uexp_extension = uexp
        .extension()
        .map(|e| e.to_string_lossy().to_string())
        .unwrap_or("".into());

    let uasset_extension = uasset
        .extension()
        .map(|e| e.to_string_lossy().to_string())
        .unwrap_or("".into());

    if uexp_extension.to_lowercase() != "uexp" || uasset_extension.to_lowercase() != "uasset" {
        return Err(anyhow::anyhow!("Filenames do not have correct extensions! Did you enter the UEXP and UASSET in the correct order?"));
    }

    let mut file = File::open(inject)?;
    let mut uexp = File::open(uexp)?;
    let mut uasset = File::open(uasset)?;

    let mut file_bytes = Vec::new();
    let mut uexp_bytes = Vec::new();
    let mut uasset_bytes = Vec::new();

    file.read_to_end(&mut file_bytes)?;
    uexp.read_to_end(&mut uexp_bytes)?;
    uasset.read_to_end(&mut uasset_bytes)?;

    let mut uexp = Cursor::new(uexp_bytes);
    let mut uasset = Cursor::new(uasset_bytes);

    // Gather data for later
    uexp.seek(SeekFrom::End(-0x4))?;
    let magic = uexp.read_u32::<LE>()?;

    println!("Got magic `{:#X}`", magic);

    let total_uasset_size = uasset.get_ref().len() as u32;
    let total_uexp_size = (uexp.get_ref().len() - 0x4) as u32;
    let total_combined_size = total_uasset_size + total_uexp_size;
    let contained_file_size = (uexp.get_ref().len() - 0x38) as u32;

    // resize to new needed size
    uexp.get_mut()
        .resize(UEXP_FILE_START + file_bytes.len() + 0x4, 0);

    uexp.set_position(UEXP_FILE_START as u64);
    uexp.write_all(&file_bytes)?;
    uexp.write_u32::<LE>(magic)?;

    let size_offset =
        find_seq(uexp.get_ref(), &contained_file_size.to_le_bytes()).unwrap_or(UEXP_SIZE_OFFSET);
    println!("Found size offset {:#X}", size_offset);

    uexp.set_position(size_offset as u64);
    let file_bytes_size = file_bytes.len() as u32;
    uexp.write_u32::<LE>(file_bytes_size)?;
    uexp.write_u32::<LE>(file_bytes_size)?;

    let uasset_total_pos =
        find_seq(uasset.get_ref(), &total_combined_size.to_le_bytes()).unwrap_or(0xA9);
    let uasset_uexp_pos = match find_seq(uasset.get_ref(), &total_uexp_size.to_le_bytes()) {
        Some(n) => n,
        None => return Err(anyhow::anyhow!("Could not find uassets uexp size description offset")),
    };

    // write uasset + uexp size
    let new_total_size = (uexp.get_ref().len() + uasset.get_ref().len() - 0x4) as u32;
    println!("new total uasset + uexp size: {:#X}", new_total_size);
    uasset.set_position(uasset_total_pos as u64);
    uasset.write_u32::<LE>(new_total_size)?;

    // write uexp size
    let new_uexp_size = (uexp.get_ref().len() - 0x4) as u32;
    println!("new uexp size: {:#X}", new_uexp_size);
    uasset.set_position(uasset_uexp_pos as u64);
    uasset.write_u32::<LE>(new_uexp_size)?;

    let mut new_uasset = File::create(uasset_path)?;
    new_uasset.write_all(uasset.get_ref())?;

    let mut new_uexp = File::create(uexp_path)?;
    new_uexp.write_all(uexp.get_ref())?;

    Ok(())
}

fn find_seq(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}
