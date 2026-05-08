#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectProfile {
    pub include_ml_ai: bool,
    pub include_qmd: bool,
    pub is_existing: bool,
}
