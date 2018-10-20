use cargo::core::dependency::Kind;
use cargo::core::{Dependency, FeatureValue::*, Manifest};

use std::collections::BTreeMap;

pub fn create_self_dependency(manifest: &Manifest) -> Dependency {
    let version = format!("={}", manifest.version());
    let mut dep = Dependency::parse_no_deprecated(
        &manifest.name(),
        Some(&version),
        manifest.summary().source_id(),
    ).unwrap();
    dep.set_default_features(false);
    dep
}

pub fn dependencies_by_feature(manifest: &Manifest) -> BTreeMap<&str, Vec<Dependency>> {
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
        let mut selfdep = create_self_dependency(&manifest);
        selfdep.set_features(&features);
        deps.insert(0, selfdep);
        deps_by_feature.insert(feature.as_str(), deps);
    }

    // calculate dependencies of optional dependencies that are also features
    let deps_required = deps_by_name
        .iter()
        .filter_map(|(_, &dep)| {
            if dep.is_optional() {
                let mut selfdep = create_self_dependency(&manifest);
                selfdep.set_features(&[""]);
                deps_by_feature.insert(&dep.package_name().as_str(), vec![selfdep, dep.clone()]);
                None
            } else {
                Some(dep.clone())
            }
        }).collect();

    deps_by_feature.insert("", deps_required);

    if !deps_by_feature.contains_key("default") {
        let mut selfdep = create_self_dependency(&manifest);
        selfdep.set_features(&[""]);
        deps_by_feature.insert("default", vec![selfdep]);
    }

    deps_by_feature
}
