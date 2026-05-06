use anyhow::Result;

use crate::cli::InitArgs;
use crate::init::answers::from_args;
use crate::init::profile::ProjectProfile;
use crate::init::scaffold::create_project;

pub fn run(args: &InitArgs) -> Result<()> {
    let answers = from_args(args)?;
    let profile = ProjectProfile::resolve(&answers.project_type, &answers.scale, answers.existing)?;
    create_project(&args.path, &answers, &profile, &args.initial_sources)
}
