extern crate cargo;
#[macro_use]
extern crate clap;
#[macro_use]
extern crate failure;
extern crate rust2rpm;

use cargo::core::SourceId;
use cargo::util::toml::read_manifest;
use cargo::Config;
use failure::Error;
use rust2rpm::crates::{create_self_dependency, dependencies_by_feature};
use rust2rpm::rpm::rpm_dep;

use std::path::Path;

fn main() -> Result<(), Error> {
    let m = clap_app!(cargo_dependency_generator =>
        (version: crate_version!())
        (@group attribute =>
            (@attributes +required)
            (@arg provides: -P --provides)
            (@arg requires: -R --requires)
        )
        (@arg feature: -f --feature +takes_value "Feature name")
        (@arg path: +takes_value +required "Path to Cargo.toml file")
    ).get_matches();

    let path = Path::new(m.value_of("path").unwrap());

    // Assuming that it comes from crates.io
    let config = Config::default()?;
    let source_id = SourceId::crates_io(&config)?;

    use cargo::core::EitherManifest;
    let manifest = match read_manifest(&path, &source_id, &config)?.0 {
        EitherManifest::Real(m) => m,
        _ => bail!("Found non-Real manifest"),
    };

    let feature = m.value_of("feature").unwrap_or("");
    let deps_by_feature = dependencies_by_feature(&manifest);
    let deps = match deps_by_feature.get(feature) {
        Some(deps) => deps,
        None => bail!("Feature {} doesn't exist"),
    };

    if m.is_present("provides") {
        let mut selfdep = create_self_dependency(&manifest);
        selfdep.set_features(&[feature]);
        println!("{}", rpm_dep(&selfdep)?);
    }

    if m.is_present("requires") {
        for dep in deps {
            println!("{}", rpm_dep(dep)?);
        }
    }

    Ok(())
}
