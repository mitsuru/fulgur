use clap::{Parser, Subcommand};
use fulgur_core::config::{Margin, PageSize};
use fulgur_core::engine::Engine;
use fulgur_core::pageable::{Pageable, SpacerPageable};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "fulgur", version, about = "HTML to PDF converter")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Render HTML to PDF (currently: test mode with placeholder content)
    Render {
        /// Output PDF file path
        #[arg(short, long)]
        output: PathBuf,

        /// Page size (A4, Letter, A3)
        #[arg(short, long, default_value = "A4")]
        size: String,

        /// Landscape orientation
        #[arg(short, long, default_value_t = false)]
        landscape: bool,

        /// PDF title
        #[arg(long)]
        title: Option<String>,
    },
}

fn parse_page_size(s: &str) -> PageSize {
    match s.to_uppercase().as_str() {
        "A4" => PageSize::A4,
        "A3" => PageSize::A3,
        "LETTER" => PageSize::LETTER,
        _ => {
            eprintln!("Unknown page size '{}', defaulting to A4", s);
            PageSize::A4
        }
    }
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Render { output, size, landscape, title } => {
            let mut builder = Engine::builder()
                .page_size(parse_page_size(&size))
                .margin(Margin::uniform_mm(20.0))
                .landscape(landscape);

            if let Some(title) = title {
                builder = builder.title(title);
            }

            let engine = builder.build();

            // For now, render a placeholder PDF (no HTML parsing yet)
            let mut spacer = SpacerPageable::new(100.0);
            spacer.wrap(100.0, 1000.0);
            let root = fulgur_core::pageable::BlockPageable::new(vec![Box::new(spacer)]);

            match engine.render_pageable_to_file(Box::new(root), &output) {
                Ok(()) => println!("PDF written to {}", output.display()),
                Err(e) => eprintln!("Error: {e}"),
            }
        }
    }
}
