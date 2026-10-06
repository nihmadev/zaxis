use super::*;

const BODY: &str =
    "fn material(in: MaterialInput, p: Params) -> vec4<f32> { return vec4<f32>(0.0); }\n";

#[test]
fn registration_is_idempotent_and_does_not_grow() {
    let mut c = setup(1.0);
    let first = c.register_material(&glow());
    for _ in 0..500 {
        assert_eq!(c.register_material(&glow()), first);
    }
    assert_eq!(c.shared_resources().material_count(), 1);
    assert!(c.material_error(first).is_none());
    // The label is not part of the identity; the source, schema and flags are.
    let renamed = Material::new("other name", GLOW)
        .param("center", ParamKind::Vec2)
        .param("color", ParamKind::Color);
    assert_eq!(c.register_material(&renamed), first);
    assert_ne!(c.register_material(&glow().animated()), first);
    let retyped = Material::new("glow", GLOW)
        .param("center", ParamKind::Vec2)
        .param("color", ParamKind::Vec4);
    assert_ne!(c.register_material(&retyped), first);
    assert_eq!(c.shared_resources().material_count(), 3);
}

#[test]
fn windows_of_one_application_share_ids() {
    let resources = SharedResources::new();
    let a = resources.register_material(&glow());
    let mut second = Context::with_shared(&resources);
    assert_eq!(second.register_material(&glow()), a);
    assert_eq!(resources.material_count(), 1);
}

#[test]
fn invalid_wgsl_is_an_error_with_a_position_not_a_panic() {
    let mut c = setup(1.0);
    let broken = Material::new(
        "broken",
        "fn material(in: MaterialInput, p: Params) -> vec4<f32> {\n    return vec4<f32>(1.0, 2.0;\n}\n",
    );
    let error = c.try_register_material(&broken).unwrap_err();
    assert_eq!(error.kind(), MaterialErrorKind::Syntax);
    assert_eq!(error.label(), "broken");
    assert_eq!(error.line(), Some(2), "{error}");
    assert!(error.column().is_some());
    assert!(error.to_string().contains("broken"));
    // Asking again returns the same remembered error without compiling anew.
    assert_eq!(c.try_register_material(&broken).unwrap_err(), error);
    assert_eq!(c.shared_resources().material_count(), 1);
}

#[test]
fn type_errors_and_a_missing_entry_function_are_reported() {
    let mut c = setup(1.0);
    let type_error = Material::new(
        "types",
        "fn material(in: MaterialInput, p: Params) -> vec4<f32> {\n    return 1.0;\n}\n",
    );
    let error = c.try_register_material(&type_error).unwrap_err();
    assert!(error.message().contains("convert"), "{error}");
    let missing = Material::new("missing", "fn other() -> f32 { return 1.0; }");
    let error = c.try_register_material(&missing).unwrap_err();
    assert!(error.message().contains("fn material"), "{error}");
}

#[test]
fn a_material_cannot_bring_its_own_resources_overrides_or_entry_points() {
    let mut c = setup(1.0);
    for (name, extra) in [
        (
            "binding",
            "@group(0) @binding(5) var<uniform> stolen: vec4<f32>;\n",
        ),
        (
            "rebinding",
            "@group(3) @binding(1) var<uniform> extra: vec4<f32>;\n",
        ),
        ("override", "override scale: f32 = 1.0;\n"),
        (
            "entry",
            "@fragment fn fs_other() -> @location(0) vec4<f32> { return vec4<f32>(1.0); }\n",
        ),
    ] {
        let error = c
            .try_register_material(&Material::new(name, format!("{extra}{BODY}")))
            .unwrap_err();
        assert_eq!(
            error.kind(),
            MaterialErrorKind::Forbidden,
            "{name}: {error}"
        );
    }
}

#[test]
fn schema_errors_are_rejected_before_compilation() {
    let mut c = setup(1.0);
    for (name, params) in [
        (
            "duplicate",
            vec![("a", ParamKind::F32), ("a", ParamKind::F32)],
        ),
        ("reserved", vec![("z_hidden", ParamKind::F32)]),
        ("digit", vec![("1a", ParamKind::F32)]),
        ("empty array", vec![("a", ParamKind::Vec4Array(0))]),
        (
            "too large",
            vec![("a", ParamKind::Vec4Array(MAX_PARAM_BYTES / 16 + 1))],
        ),
    ] {
        let mut material = Material::new(name, BODY);
        for (n, kind) in params {
            material = material.param(n, kind);
        }
        let error = c.try_register_material(&material).unwrap_err();
        assert_eq!(error.kind(), MaterialErrorKind::Schema, "{name}: {error}");
    }
}

#[test]
fn registry_is_bounded() {
    let resources = SharedResources::new();
    let numbered = |n: i32| {
        Material::new(
            "n",
            format!(
                "fn material(in: MaterialInput, p: Params) -> vec4<f32> {{ return vec4<f32>({n}.0); }}"
            ),
        )
    };
    for n in 0..MAX_MATERIALS as i32 {
        resources.try_register_material(&numbered(n)).unwrap();
    }
    assert_eq!(resources.material_count(), MAX_MATERIALS);
    let one_more = numbered(-1);
    let error = resources.try_register_material(&one_more).unwrap_err();
    assert_eq!(error.kind(), MaterialErrorKind::Limit);
    assert_eq!(resources.register_material(&one_more), MaterialId::NONE);
    assert_eq!(resources.material_count(), MAX_MATERIALS);
    // A removed material makes room.
    let id = resources.try_register_material(&numbered(0)).unwrap();
    assert!(resources.remove_material(id));
    assert!(resources.try_register_material(&one_more).is_ok());
}

#[test]
fn a_rejected_registration_is_reported_once_through_diagnostics() {
    let mut c = setup(1.0);
    let reports = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let sink = reports.clone();
    c.set_diagnostic_handler(Some(Box::new(move |d| sink.borrow_mut().push(d.clone()))));
    let broken = Material::new(
        "broken",
        "fn material(in: MaterialInput, p: Params) -> vec4<f32> { return 1.0; }",
    );
    let id = c.register_material(&broken);
    for _ in 0..5 {
        c.run(|c| {
            c.register_material(&broken);
        });
    }
    assert!(c.material_error(id).is_some());
    let reports = reports.borrow();
    assert_eq!(reports.len(), 1, "{reports:?}");
    assert_eq!(reports[0].kind, DiagnosticKind::InvalidShader);
    assert!(reports[0].message.contains("broken"));
}
