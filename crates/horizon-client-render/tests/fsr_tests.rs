const FSR_WGSL: &str = include_str!("../src/gfx/fsr.wgsl");

#[test]
fn fsr_wgsl_parses_and_validates() {
    let module = match naga::front::wgsl::parse_str(FSR_WGSL) {
        Ok(m) => m,
        Err(e) => {
            panic!("WGSL parsing failed: {}", e.emit_to_string(FSR_WGSL));
        }
    };

    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    );
    match validator.validate(&module) {
        Ok(_) => (),
        Err(e) => {
            panic!("WGSL validation failed: {:?}", e);
        }
    }

    // Verify entry points exist
    let entry_points: Vec<&str> = module.entry_points.iter().map(|ep| ep.name.as_str()).collect();
    assert!(
        entry_points.contains(&"vs_fsr"),
        "Missing vs_fsr entry point in fsr.wgsl; found: {:?}",
        entry_points
    );
    assert!(
        entry_points.contains(&"fs_easu"),
        "Missing fs_easu entry point in fsr.wgsl; found: {:?}",
        entry_points
    );
    assert!(
        entry_points.contains(&"fs_rcas"),
        "Missing fs_rcas entry point in fsr.wgsl; found: {:?}",
        entry_points
    );
}

#[test]
fn fsr_constants_layout_matches_spec() {
    let module = naga::front::wgsl::parse_str(FSR_WGSL).expect("fsr.wgsl parse");
    let mut found_constants = false;
    for (_, ty) in module.types.iter() {
        if let Some(name) = &ty.name {
            if name == "FsrConstants" {
                found_constants = true;
                if let naga::TypeInner::Struct { members, .. } = &ty.inner {
                    let member_names: Vec<&str> = members
                        .iter()
                        .filter_map(|m| m.name.as_deref())
                        .collect();
                    assert_eq!(
                        member_names,
                        vec!["src_size", "dst_size", "sharpness", "_pad"],
                        "FsrConstants struct members do not match spec"
                    );
                } else {
                    panic!("FsrConstants is not a struct");
                }
            }
        }
    }
    assert!(found_constants, "FsrConstants struct not found in module");
}
