use std::fmt;
use std::str::FromStr;

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use super::packs::Pack;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Blueprint {
    Generic,
    WebProduct,
    LibrarySdk,
    MlResearch,
    OpsInfra,
    Security,
    Research,
    Custom,
}

impl Blueprint {
    pub const ALL: &'static [Blueprint] = &[
        Blueprint::Generic,
        Blueprint::WebProduct,
        Blueprint::LibrarySdk,
        Blueprint::MlResearch,
        Blueprint::OpsInfra,
        Blueprint::Security,
        Blueprint::Research,
        Blueprint::Custom,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Blueprint::Generic => "generic",
            Blueprint::WebProduct => "web-product",
            Blueprint::LibrarySdk => "library-sdk",
            Blueprint::MlResearch => "ml-research",
            Blueprint::OpsInfra => "ops-infra",
            Blueprint::Security => "security",
            Blueprint::Research => "research",
            Blueprint::Custom => "custom",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Blueprint::Generic => "Spine-only knowledge base with no domain assumptions",
            Blueprint::WebProduct => {
                "Frontend, backend, or full-stack product with users and releases"
            }
            Blueprint::LibrarySdk => "Public API surface with semver and external consumers",
            Blueprint::MlResearch => "Pre-production model work: experiments, datasets, evals",
            Blueprint::OpsInfra => "Infrastructure or platform work: runbooks and incidents",
            Blueprint::Security => "Security review, threat modeling, audit, or finding tracking",
            Blueprint::Research => "Non-ML research: literature notes, hypotheses, citations",
            Blueprint::Custom => "Empty default; choose packs manually",
        }
    }

    pub fn default_packs(self) -> &'static [Pack] {
        match self {
            Blueprint::Generic | Blueprint::Custom => &[],
            Blueprint::WebProduct => &[Pack::Api, Pack::Frontend, Pack::OpsLite],
            Blueprint::LibrarySdk => &[Pack::Api, Pack::Library],
            Blueprint::MlResearch => &[Pack::Ml, Pack::Data, Pack::Research],
            Blueprint::OpsInfra => &[Pack::Ops],
            Blueprint::Security => &[Pack::Security, Pack::OpsLite],
            Blueprint::Research => &[Pack::Research, Pack::QmdRsScale],
        }
    }
}

impl fmt::Display for Blueprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for Blueprint {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        let normalized = value.trim().to_ascii_lowercase();
        for blueprint in Blueprint::ALL {
            if blueprint.name() == normalized {
                return Ok(*blueprint);
            }
        }
        bail!("unknown blueprint: {value}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_blueprints_round_trip_through_name() {
        for blueprint in Blueprint::ALL {
            assert_eq!(blueprint.name().parse::<Blueprint>().unwrap(), *blueprint);
            assert_eq!(blueprint.to_string(), blueprint.name());
        }
    }

    #[test]
    fn default_packs_are_in_catalog() {
        for blueprint in Blueprint::ALL {
            assert!(!blueprint.description().is_empty());
            for pack in blueprint.default_packs() {
                assert!(
                    Pack::ALL.contains(pack),
                    "{} references unknown pack {}",
                    blueprint.name(),
                    pack.name()
                );
            }
        }
    }
}
