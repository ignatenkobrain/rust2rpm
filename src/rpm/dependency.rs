use cargo::core::Dependency;
use failure::Error;
use semver_parser::range::{self, Op::*, Predicate, WildcardVersion};

use std::fmt;

#[derive(Clone, Debug)]
enum V {
    M(u64),
    MM(u64, u64),
    MMP(u64, u64, u64),
}

use self::V::*;

impl V {
    fn new(p: &Predicate) -> Result<Self, Error> {
        let mmp = match (p.minor, p.patch) {
            (None, None) => M(p.major),
            (Some(minor), None) => MM(p.major, minor),
            (Some(minor), Some(patch)) => MMP(p.major, minor, patch),
            (None, Some(_)) => bail!("semver had patch without minor"),
        };
        Ok(mmp)
    }

    fn major(&self) -> u64 {
        match *self {
            M(major) | MM(major, _) | MMP(major, _, _) => major,
        }
    }

    fn minor(&self) -> u64 {
        match *self {
            M(_) => 0,
            MM(_, minor) | MMP(_, minor, _) => minor,
        }
    }

    fn patch(&self) -> u64 {
        match *self {
            M(_) | MM(_, _) => 0,
            MMP(_, _, patch) => patch,
        }
    }
}

impl fmt::Display for V {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major(), self.minor(), self.patch())
    }
}

fn apply_version_range(cap: &str, reqs: &Vec<(&str, V)>) -> String {
    let deps: Vec<_> = reqs
        .iter()
        .map(|(op, ver)| format!("{} {} {}", cap, op, ver))
        .collect();

    match deps.len() {
        0 => cap.to_string(),
        1 => unsafe { deps.get_unchecked(0).to_string() },
        _ => format!("({})", deps.join(" with ")),
    }
}

pub fn rpm_dep(dependency: &Dependency) -> Result<String, Error> {
    let req = range::parse(&dependency.version_req().to_string()).unwrap();
    let mut reqs = Vec::new();
    for p in &req.predicates {
        if p.pre.len() > 0 {
            unimplemented!();
        }
        let mmp = V::new(p)?;
        match (&p.op, &mmp) {
            (Lt, _) => {
                reqs.push(("<", mmp.clone()));
            }
            (LtEq, _) => {
                reqs.push(("<=", mmp.clone()));
            }
            (Gt, _) => {
                reqs.push((">", mmp.clone()));
            }
            (GtEq, _) => {
                reqs.push(("<=", mmp.clone()));
            }
            (Ex, _) => {
                reqs.push(("=", mmp.clone()));
            }
            (Compatible, &MMP(0, 0, patch)) => {
                reqs.push((">=", mmp.clone()));
                reqs.push(("<", MMP(0, 0, patch + 1)));
            }
            (Compatible, &MM(0, minor)) | (Compatible, &MMP(0, minor, _)) => {
                reqs.push((">=", mmp.clone()));
                reqs.push(("<", MM(0, minor + 1)));
            }
            (Tilde, &MM(major, minor)) | (Tilde, &MMP(major, minor, _)) => {
                reqs.push((">=", mmp.clone()));
                reqs.push(("<", MM(major, minor + 1)));
            }
            (Compatible, &M(major))
            | (Tilde, &M(major))
            | (Compatible, &MM(major, _))
            | (Compatible, &MMP(major, _, _)) => {
                reqs.push((">=", mmp.clone()));
                reqs.push(("<", M(major + 1)));
            }
            (Wildcard(WildcardVersion::Minor), _) => {
                reqs.push((">=", M(mmp.major())));
                reqs.push(("<", M(mmp.major() + 1)));
            }
            (Wildcard(WildcardVersion::Patch), _) => {
                reqs.push((">=", mmp.clone()));
                reqs.push(("<", MM(mmp.major(), mmp.minor() + 1)));
            }
        }
    }

    let name = dependency.package_name();
    let mut caps = Vec::new();
    if dependency.uses_default_features() {
        caps.push(format!("crate({}/default)", name));
    }
    for feature in dependency.features() {
        caps.push(match feature.as_str() {
            "" => format!("crate({})", name),
            _ => format!("crate({}/{})", name, feature),
        });
    }
    if caps.is_empty() {
        caps.push(format!("crate({})", name));
    }

    let rpm_deps: Vec<_> = caps
        .iter()
        .map(|cap| apply_version_range(cap, &reqs))
        .collect();
    let depstr = match rpm_deps.len() {
        1 => unsafe { rpm_deps.get_unchecked(0).to_string() },
        _ => format!("({})", rpm_deps.join(" and ")),
    };
    Ok(depstr)
}
