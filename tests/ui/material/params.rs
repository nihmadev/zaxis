use super::*;

fn layout(params: &[(&str, ParamKind)]) -> ParamLayout {
    let params: Vec<_> = params.iter().map(|(n, k)| (n.to_string(), *k)).collect();
    ParamLayout::new(&params).unwrap()
}

#[test]
fn offsets_follow_the_uniform_rules() {
    let l = layout(&[
        ("a", ParamKind::F32),
        ("v3", ParamKind::Vec3),
        ("tail", ParamKind::F32),
        ("v2", ParamKind::Vec2),
        ("m2", ParamKind::Mat2),
        ("m3", ParamKind::Mat3),
        ("v4", ParamKind::Vec4),
        ("arr", ParamKind::Vec4Array(3)),
    ]);
    assert_eq!(l.offset_of("a"), Some(0));
    // A vec3 is 16-aligned and 12 bytes: the next scalar sits in its tail.
    assert_eq!(l.offset_of("v3"), Some(16));
    assert_eq!(l.offset_of("tail"), Some(28));
    assert_eq!(l.offset_of("v2"), Some(32));
    assert_eq!(l.offset_of("m2"), Some(40));
    assert_eq!(l.offset_of("m3"), Some(64));
    assert_eq!(l.offset_of("v4"), Some(112));
    assert_eq!(l.offset_of("arr"), Some(128));
    assert_eq!(l.size(), 176);
    assert_eq!(l.size() % 16, 0);
    assert_eq!(l.block_size(), FRAME_BYTES + 176);
}

#[test]
fn values_are_packed_where_the_shader_reads_them() {
    let l = layout(&[
        ("a", ParamKind::F32),
        ("v3", ParamKind::Vec3),
        ("tail", ParamKind::U32),
        ("m3", ParamKind::Mat3),
        ("n", ParamKind::I32),
    ]);
    let bytes = Params::new()
        .f32("a", 1.5)
        .vec3("v3", [2.0, 3.0, 4.0])
        .u32("tail", 7)
        .mat3("m3", [[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]])
        .i32("n", -3)
        .pack(&l)
        .unwrap();
    let f = |at: usize| f32::from_ne_bytes(bytes[at..at + 4].try_into().unwrap());
    assert_eq!(bytes.len(), l.size());
    assert_eq!([f(0), f(16), f(20), f(24)], [1.5, 2.0, 3.0, 4.0]);
    assert_eq!(u32::from_ne_bytes(bytes[28..32].try_into().unwrap()), 7);
    // Columns of a mat3 are 16 bytes apart; the fourth float of each column is padding.
    assert_eq!([f(32), f(36), f(40), f(44)], [1.0, 2.0, 3.0, 0.0]);
    assert_eq!([f(48), f(52), f(56)], [4.0, 5.0, 6.0]);
    assert_eq!([f(64), f(68), f(72)], [7.0, 8.0, 9.0]);
    assert_eq!(i32::from_ne_bytes(bytes[80..84].try_into().unwrap()), -3);
}

#[test]
fn unset_parameters_are_zero_and_so_are_non_finite_floats() {
    let l = layout(&[("a", ParamKind::F32), ("b", ParamKind::F32)]);
    let bytes = Params::new().f32("a", f32::NAN).pack(&l).unwrap();
    assert!(bytes.iter().all(|b| *b == 0));
}

#[test]
fn the_uniform_limit_is_enforced() {
    let fits = ParamKind::Vec4Array(MAX_PARAM_BYTES / 16);
    assert!(ParamLayout::new(&[("a".into(), fits)]).is_ok());
    let error = ParamLayout::new(&[("a".into(), fits), ("b".into(), ParamKind::F32)]).unwrap_err();
    assert_eq!(error.kind(), MaterialErrorKind::Schema);
    assert!(
        error.message().contains(&MAX_UNIFORM_BYTES.to_string()),
        "{error}"
    );
    assert_eq!(FRAME_BYTES + MAX_PARAM_BYTES, MAX_UNIFORM_BYTES);
}

#[test]
fn mismatched_values_are_errors() {
    let l = layout(&[("a", ParamKind::F32), ("arr", ParamKind::Vec4Array(2))]);
    assert!(Params::new().vec2("a", vec2(1.0, 2.0)).pack(&l).is_err());
    assert!(Params::new().f32("missing", 1.0).pack(&l).is_err());
    assert!(Params::new().array("arr", &[[0.0; 4]; 3]).pack(&l).is_err());
    assert!(Params::new().array("arr", &[[1.0; 4]; 2]).pack(&l).is_ok());
}

#[test]
fn every_kind_compiles_with_the_layout_the_cpu_packs() {
    // Registration compares the offsets the shader front end computed with ours.
    let mut c = setup(1.0);
    let material = Material::new(
        "all kinds",
        "fn material(in: MaterialInput, p: Params) -> vec4<f32> {\n\
         let s = p.a + f32(p.b) + f32(p.c) + p.d.x + p.e.x + p.f.x + p.g.x + p.h[0][0] + p.i[0][0] + p.j[0][0] + p.k[1].x;\n\
         return vec4<f32>(s); }",
    )
    .param("a", ParamKind::F32)
    .param("b", ParamKind::I32)
    .param("c", ParamKind::U32)
    .param("d", ParamKind::Vec2)
    .param("e", ParamKind::Vec3)
    .param("f", ParamKind::Vec4)
    .param("g", ParamKind::Color)
    .param("h", ParamKind::Mat2)
    .param("i", ParamKind::Mat3)
    .param("j", ParamKind::Mat4)
    .param("k", ParamKind::Vec4Array(4));
    c.try_register_material(&material).unwrap();
}
