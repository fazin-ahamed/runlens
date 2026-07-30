use crate::paths::WorkspacePaths;
use anyhow::Result;
use clap::Parser;

#[derive(Debug, Parser)]
pub struct DoctorArgs {
    #[arg(long)]
    pub json: bool,
    #[arg(long)]
    pub bundle: Option<String>,
}

pub async fn run(_workspace: &WorkspacePaths, args: &DoctorArgs) -> Result<()> {
    let doctor = runlens_doctor::Doctor::new();
    let report = doctor.run_all();
    if args.json {
        println!("{}", runlens_doctor::format_json(&report));
    } else {
        print!("{}", runlens_doctor::format_report(&report));
    }
    Ok(())
}
