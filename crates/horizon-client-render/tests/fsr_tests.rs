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
    let entry_points: Vec<&str> = module
        .entry_points
        .iter()
        .map(|ep| ep.name.as_str())
        .collect();
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
        if ty.name.as_deref() != Some("FsrConstants") {
            continue;
        }
        found_constants = true;
        let naga::TypeInner::Struct { members, .. } = &ty.inner else {
            panic!("FsrConstants is not a struct");
        };
        let member_names: Vec<&str> = members.iter().filter_map(|m| m.name.as_deref()).collect();
        assert_eq!(
            member_names,
            vec!["src_size", "dst_size", "sharpness", "_pad"],
            "FsrConstants struct members do not match spec"
        );
    }
    assert!(found_constants, "FsrConstants struct not found in module");
}

#[test]
fn fsr_constants_rust_struct_layout() {
    use horizon_client_render::gfx::fsr::FsrConstants;
    use std::mem::{align_of, size_of};

    assert_eq!(size_of::<FsrConstants>(), 24);
    assert_eq!(align_of::<FsrConstants>(), 4);

    let constants = FsrConstants {
        src_size: [1280.0, 720.0],
        dst_size: [1920.0, 1080.0],
        sharpness: 0.2,
        _pad: 0.0,
    };

    let bytes = bytemuck::bytes_of(&constants);
    assert_eq!(bytes.len(), 24);

    let decoded: FsrConstants = *bytemuck::from_bytes(bytes);
    assert_eq!(decoded, constants);
}
