use cargo::core::dependency::Kind;
use cargo::core::{Dependency, FeatureValue::*, Manifest};

use std::collections::BTreeMap;

//type DependenciesByFeature = BTreeMap<&str, (Vec<&str>, Vec<Dependency>)>;

pub fn create_self_dependency(manifest: &Manifest, features: &[&str]) -> Dependency {
    let version = format!("={}", manifest.version());
    let mut dep = Dependency::parse_no_deprecated(
        &manifest.name(),
        Some(&version),
        manifest.summary().source_id(),
    ).unwrap();
    dep.set_default_features(false);
    dep.set_features(features);
    dep
}

/// For a given feature, find all other features it uses and combine
/// all their dependencies.
pub fn resolve_dependencies_for_feature<'a>(
    deps_by_feature: &'a BTreeMap<&str, (Vec<&str>, Vec<Dependency>)>,
    feature: &str,
) -> (Vec<&'a str>, Vec<Dependency>) {
    let mut all_features = Vec::new();
    let mut all_deps = Vec::new();
    let &(ref ff, ref dd) = deps_by_feature.get(feature).unwrap();
    all_features.extend(ff.clone());
    all_deps.extend(dd.clone());
    for f in ff {
        // Features can't create cycles, so it is safe.
        let (ff1, dd1) = resolve_dependencies_for_feature(&deps_by_feature, f);
        all_features.extend(ff1);
        all_deps.extend(dd1);
    }
    (all_features, all_deps)
}

pub fn dependencies_by_feature(
    manifest: &Manifest,
) -> BTreeMap<&str, (Vec<&str>, Vec<Dependency>)> {
    let deps_by_name: BTreeMap<&str, &Dependency> = manifest
        .dependencies()
        .iter()
        .filter_map(|dep| {
            // Features can reference optional deps
            // and development ones can't be optional
            if dep.kind() == Kind::Development {
                None
            } else {
                Some((dep.package_name().as_str(), dep))
            }
        }).collect();

    let mut deps_by_feature = BTreeMap::new();
    for (feature, f_deps) in manifest.summary().features() {
        let mut features = vec![""];
        let mut deps = Vec::new();
        for dep in f_deps {
            match dep {
                Feature(feature_name) => features.push(feature_name),
                Crate(crate_name) => {
                    let dep = deps_by_name.get(crate_name.as_str()).unwrap();
                    deps.push((*dep).clone());
                }
                CrateFeature(crate_name, crate_feature) => {
                    let dep = deps_by_name.get(crate_name.as_str()).unwrap();
                    let mut dep = (*dep).clone();
                    dep.set_default_features(false);
                    dep.set_features(vec![crate_feature.to_string()]);
                    deps.push(dep);
                }
            }
        }
        deps_by_feature.insert(feature.as_str(), (features, deps));
    }

    // calculate dependencies of optional dependencies that are also features
    let deps_required = deps_by_name
        .iter()
        .filter_map(|(_, &dep)| {
            if dep.is_optional() {
                deps_by_feature.insert(&dep.package_name().as_str(), (vec![""], vec![dep.clone()]));
                None
            } else {
                Some(dep.clone())
            }
        }).collect();

    deps_by_feature.insert("", (vec![], deps_required));

    if !deps_by_feature.contains_key("default") {
        deps_by_feature.insert("default", (vec![""], vec![]));
    }

    deps_by_feature
}
