use crate::paths::WorkspacePaths;
use anyhow::Result;
use clap::Parser;

#[derive(Debug, Parser)]
pub struct DiagnoseArgs {
    pub session_id: String,
    pub question: Vec<String>,
    #[arg(long)]
    pub json: bool,
}

pub async fn run(_workspace: &WorkspacePaths, args: &DiagnoseArgs) -> Result<()> {
    let question = args.question.join(" ");
    let mut engine = runlens_diagnosis::DiagnosisEngine::new();
    engine.add_evidence(&args.session_id, vec![]);
    let diagnosis = engine.diagnose(&args.session_id, &question);
    if args.json {
        println!("{}", serde_json::to_string_pretty(&diagnosis)?);
    } else {
        println!("Diagnosis: {}", diagnosis.answer);
        println!("Confidence: {:.2}", diagnosis.confidence);
    }
    Ok(())
}
