use std::{collections::BTreeMap, panic::AssertUnwindSafe};

use kinetik_ui_core::{Size, TextureId};
use kinetik_ui_render::{RenderImageSampling, TextureResource};
use vello::wgpu;

use crate::{
    PresenterDeviceScope, VelloNativeTextureUpdateOutcome, VelloNativeTextureValidationError,
    VelloPresenterError,
    device::DeviceAuthority,
    presenter::{test_detached_native_scope_guards, test_foreign_and_stale_native_scope_guards},
};

use super::{
    NativeTextureDescriptor, NativeTextureMutationDriver, NativeTextureOperations,
    VelloNativeTextureRegistration,
};

#[derive(Default)]
struct FakeOperations {
    trace: Vec<&'static str>,
    active: BTreeMap<TextureId, ([u32; 2], RenderImageSampling)>,
    pending: BTreeMap<TextureId, ([u32; 2], RenderImageSampling)>,
    panic_on: Option<&'static str>,
    clear_calls: usize,
}

impl FakeOperations {
    fn step(&mut self, name: &'static str) {
        self.trace.push(name);
        assert_ne!(self.panic_on, Some(name), "test backend step");
    }
}

impl NativeTextureOperations for FakeOperations {
    fn invalidate(&mut self, texture: TextureId) -> Result<(), VelloPresenterError> {
        self.step("invalidate");
        let value = self
            .active
            .remove(&texture)
            .ok_or(VelloPresenterError::StaleNativeTextureRegistration { texture })?;
        let _ = self.pending.insert(texture, value);
        Ok(())
    }

    fn register_texture(
        &mut self,
        texture_id: TextureId,
        extent: [u32; 2],
        sampling: RenderImageSampling,
    ) -> Result<(), VelloPresenterError> {
        self.step("register");
        if self.active.contains_key(&texture_id) || self.pending.contains_key(&texture_id) {
            return Err(VelloPresenterError::NativeTextureAlreadyRegistered {
                texture: texture_id,
            });
        }
        let _ = self.pending.insert(texture_id, (extent, sampling));
        Ok(())
    }

    fn mark_override_image_dirty(&mut self, texture: TextureId) -> Result<(), VelloPresenterError> {
        self.step("dirty");
        if self.pending.contains_key(&texture) {
            Ok(())
        } else {
            Err(VelloPresenterError::StaleNativeTextureRegistration { texture })
        }
    }

    fn override_image(
        &mut self,
        texture_id: TextureId,
        extent: [u32; 2],
        sampling: RenderImageSampling,
    ) -> Result<(), VelloPresenterError> {
        self.step("override");
        let entry = self.pending.get_mut(&texture_id).ok_or(
            VelloPresenterError::StaleNativeTextureRegistration {
                texture: texture_id,
            },
        )?;
        *entry = (extent, sampling);
        Ok(())
    }

    fn unregister_texture(&mut self, texture: TextureId) -> Result<(), VelloPresenterError> {
        self.step("unregister");
        self.pending
            .remove(&texture)
            .map(|_| ())
            .ok_or(VelloPresenterError::StaleNativeTextureRegistration { texture })
    }

    fn publish(&mut self, texture: TextureId) -> Result<(), VelloPresenterError> {
        self.step("publish");
        let value = self
            .pending
            .remove(&texture)
            .ok_or(VelloPresenterError::StaleNativeTextureRegistration { texture })?;
        let _ = self.active.insert(texture, value);
        Ok(())
    }

    fn clear(&mut self) {
        self.step("clear");
        self.active.clear();
        self.pending.clear();
        self.clear_calls += 1;
    }
}

fn scope(presenter: u64, generation: u64) -> PresenterDeviceScope {
    let mut authority = DeviceAuthority::for_test(presenter, generation, false);
    authority.activate()
}

fn resource(texture: TextureId, width: f32, height: f32) -> TextureResource {
    TextureResource {
        id: texture,
        size: Size::new(width, height),
        sampling: RenderImageSampling::Pixelated,
        snapshot: None,
    }
}

fn descriptor(width: u32, height: u32) -> NativeTextureDescriptor {
    NativeTextureDescriptor {
        extent: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::COPY_SRC,
        dimension: wgpu::TextureDimension::D2,
        mip_level_count: 1,
        sample_count: 1,
    }
}

fn registered_fixture(
    texture: TextureId,
) -> (
    PresenterDeviceScope,
    TextureResource,
    NativeTextureMutationDriver,
    FakeOperations,
    VelloNativeTextureRegistration,
) {
    let scope = scope(8_001, 1);
    let resource = resource(texture, 2.0, 2.0);
    let mut driver = NativeTextureMutationDriver::new();
    let mut operations = FakeOperations::default();
    let registration = driver
        .register(&scope, &resource, descriptor(2, 2), 5, &mut operations)
        .expect("valid registration");
    (scope, resource, driver, operations, registration)
}

#[test]
fn native_texture_register_dirty_replace_resize_remove_reuse_is_ordered() {
    let texture = TextureId::from_raw(801);
    let (scope, base_resource, mut driver, mut operations, registration) =
        registered_fixture(texture);
    let outcome = driver
        .update(&scope, &registration, 6, &mut operations)
        .expect("new revision");
    assert_eq!(outcome, VelloNativeTextureUpdateOutcome::MarkedDirty);
    let replaced = driver
        .replace(
            &scope,
            &registration,
            &base_resource,
            descriptor(2, 2),
            6,
            &mut operations,
        )
        .expect("same extent replacement");
    let resized_resource = resource(texture, 3.0, 2.0);
    let resized = driver
        .replace(
            &scope,
            &replaced,
            &resized_resource,
            descriptor(3, 2),
            7,
            &mut operations,
        )
        .expect("resized replacement");
    driver
        .remove(&scope, &resized, &mut operations)
        .expect("remove registration");
    let reused = driver
        .register(&scope, &base_resource, descriptor(2, 2), 8, &mut operations)
        .expect("reuse removed ID");

    assert_eq!(reused.texture_id(), texture);
    assert_eq!(
        operations.trace,
        vec![
            "register",
            "publish",
            "invalidate",
            "dirty",
            "publish",
            "invalidate",
            "override",
            "publish",
            "invalidate",
            "unregister",
            "register",
            "publish",
            "invalidate",
            "unregister",
            "register",
            "publish",
        ]
    );
    assert!(operations.active.contains_key(&texture));
}

#[test]
fn unchanged_revision_is_noop_and_regressed_revision_is_atomic() {
    let texture = TextureId::from_raw(802);
    let (scope, _, mut driver, mut operations, registration) = registered_fixture(texture);
    operations.trace.clear();
    assert_eq!(
        driver
            .update(&scope, &registration, 5, &mut operations)
            .expect("equal revision"),
        VelloNativeTextureUpdateOutcome::Unchanged
    );
    assert!(operations.trace.is_empty());
    let error = driver
        .update(&scope, &registration, 4, &mut operations)
        .expect_err("lower revision");
    assert!(matches!(
        error,
        VelloPresenterError::NativeTextureRevisionRegressed {
            texture: rejected,
            current: 5,
            requested: 4,
        } if rejected == texture
    ));
    assert!(operations.trace.is_empty());
    assert!(operations.active.contains_key(&texture));

    assert_eq!(
        driver
            .update(&scope, &registration, 6, &mut operations)
            .expect("higher revision"),
        VelloNativeTextureUpdateOutcome::MarkedDirty
    );
    assert_eq!(operations.trace, vec!["invalidate", "dirty", "publish"]);
    assert!(operations.pending.is_empty());
    assert!(operations.active.contains_key(&texture));
}

#[test]
#[allow(clippy::too_many_lines)]
fn invalid_metadata_and_duplicate_registration_mutate_nothing() {
    let texture = TextureId::from_raw(803);
    let valid_resource = resource(texture, 2.0, 2.0);
    let mut cases = vec![
        (
            resource(texture, 2.0, 2.0),
            descriptor(0, 2),
            VelloNativeTextureValidationError::ZeroExtent,
        ),
        (
            resource(texture, 2.0, 2.0),
            descriptor(2, 0),
            VelloNativeTextureValidationError::ZeroExtent,
        ),
        (
            resource(texture, f32::NAN, 2.0),
            descriptor(2, 2),
            VelloNativeTextureValidationError::NonIntegralResourceExtent,
        ),
        (
            resource(texture, 2.0, f32::INFINITY),
            descriptor(2, 2),
            VelloNativeTextureValidationError::NonIntegralResourceExtent,
        ),
        (
            resource(texture, 0.0, 2.0),
            descriptor(2, 2),
            VelloNativeTextureValidationError::NonIntegralResourceExtent,
        ),
        (
            resource(texture, 2.0, 0.0),
            descriptor(2, 2),
            VelloNativeTextureValidationError::NonIntegralResourceExtent,
        ),
        (
            resource(texture, -1.0, 2.0),
            descriptor(2, 2),
            VelloNativeTextureValidationError::NonIntegralResourceExtent,
        ),
        (
            resource(texture, 2.0, -1.0),
            descriptor(2, 2),
            VelloNativeTextureValidationError::NonIntegralResourceExtent,
        ),
        (
            resource(texture, 1.5, 2.0),
            descriptor(2, 2),
            VelloNativeTextureValidationError::NonIntegralResourceExtent,
        ),
        (
            resource(texture, 2.0, 1.5),
            descriptor(2, 2),
            VelloNativeTextureValidationError::NonIntegralResourceExtent,
        ),
        (
            resource(texture, 3.0, 2.0),
            descriptor(2, 2),
            VelloNativeTextureValidationError::ResourceExtentMismatch,
        ),
        (
            resource(texture, 2.0, 3.0),
            descriptor(2, 2),
            VelloNativeTextureValidationError::ResourceExtentMismatch,
        ),
    ];
    let mut invalid_format = descriptor(2, 2);
    invalid_format.format = wgpu::TextureFormat::Bgra8Unorm;
    cases.push((
        resource(texture, 2.0, 2.0),
        invalid_format,
        VelloNativeTextureValidationError::UnsupportedFormat,
    ));
    let mut missing_usage = descriptor(2, 2);
    missing_usage.usage = wgpu::TextureUsages::TEXTURE_BINDING;
    cases.push((
        resource(texture, 2.0, 2.0),
        missing_usage,
        VelloNativeTextureValidationError::MissingCopySourceUsage,
    ));
    let mut invalid_dimension = descriptor(2, 2);
    invalid_dimension.dimension = wgpu::TextureDimension::D3;
    cases.push((
        resource(texture, 2.0, 2.0),
        invalid_dimension,
        VelloNativeTextureValidationError::UnsupportedDimension,
    ));
    let mut invalid_layers = descriptor(2, 2);
    invalid_layers.extent.depth_or_array_layers = 2;
    cases.push((
        resource(texture, 2.0, 2.0),
        invalid_layers,
        VelloNativeTextureValidationError::UnsupportedArrayLayers,
    ));
    let mut invalid_mips = descriptor(2, 2);
    invalid_mips.mip_level_count = 2;
    cases.push((
        resource(texture, 2.0, 2.0),
        invalid_mips,
        VelloNativeTextureValidationError::UnsupportedMipLevels,
    ));
    let mut invalid_samples = descriptor(2, 2);
    invalid_samples.sample_count = 2;
    cases.push((
        resource(texture, 2.0, 2.0),
        invalid_samples,
        VelloNativeTextureValidationError::UnsupportedSampleCount,
    ));

    for (invalid_resource, invalid_descriptor, reason) in cases {
        let scope = scope(8_003, 1);
        let mut driver = NativeTextureMutationDriver::new();
        let mut operations = FakeOperations::default();
        let register_error = driver
            .register(
                &scope,
                &invalid_resource,
                invalid_descriptor,
                1,
                &mut operations,
            )
            .expect_err("invalid register metadata");
        assert!(matches!(
            register_error,
            VelloPresenterError::InvalidNativeTexture {
                texture: rejected,
                reason: observed,
            } if rejected == texture && observed == reason
        ));
        assert!(operations.trace.is_empty());

        let registration = driver
            .register(
                &scope,
                &valid_resource,
                descriptor(2, 2),
                1,
                &mut operations,
            )
            .expect("valid setup registration");
        operations.trace.clear();
        let replace_error = driver
            .replace(
                &scope,
                &registration,
                &invalid_resource,
                invalid_descriptor,
                2,
                &mut operations,
            )
            .expect_err("invalid replace metadata");
        assert!(matches!(
            replace_error,
            VelloPresenterError::InvalidNativeTexture {
                texture: rejected,
                reason: observed,
            } if rejected == texture && observed == reason
        ));
        assert!(operations.trace.is_empty());
    }

    let scope = scope(8_004, 1);
    let mut driver = NativeTextureMutationDriver::new();
    let mut operations = FakeOperations::default();
    let _ = driver
        .register(
            &scope,
            &valid_resource,
            descriptor(2, 2),
            1,
            &mut operations,
        )
        .expect("first registration");
    operations.trace.clear();
    let duplicate = driver
        .register(
            &scope,
            &valid_resource,
            descriptor(2, 2),
            2,
            &mut operations,
        )
        .expect_err("duplicate registration");
    assert!(matches!(
        duplicate,
        VelloPresenterError::NativeTextureAlreadyRegistered { texture: rejected }
            if rejected == texture
    ));
    assert!(operations.trace.is_empty());
}

#[test]
fn copy_source_usage_superset_is_accepted() {
    let texture = TextureId::from_raw(8_033);
    let scope = scope(8_033, 1);
    let mut driver = NativeTextureMutationDriver::new();
    let mut operations = FakeOperations::default();
    let mut native = descriptor(2, 2);
    native.usage |= wgpu::TextureUsages::RENDER_ATTACHMENT;

    let registration = driver
        .register(
            &scope,
            &resource(texture, 2.0, 2.0),
            native,
            1,
            &mut operations,
        )
        .expect("COPY_SRC usage supersets are valid producer textures");

    assert_eq!(registration.texture_id(), texture);
    assert_eq!(operations.trace, vec!["register", "publish"]);
    assert!(operations.pending.is_empty());
    assert!(operations.active.contains_key(&texture));
}

#[test]
fn resource_id_mismatch_and_replacement_revision_rules_are_atomic() {
    let texture = TextureId::from_raw(804);
    let (scope, base_resource, mut driver, mut operations, registration) =
        registered_fixture(texture);
    operations.trace.clear();
    let mismatched = resource(TextureId::from_raw(9_804), 2.0, 2.0);
    let error = driver
        .replace(
            &scope,
            &registration,
            &mismatched,
            descriptor(2, 2),
            5,
            &mut operations,
        )
        .expect_err("resource ID mismatch");
    assert!(matches!(
        error,
        VelloPresenterError::InvalidNativeTexture {
            texture: rejected,
            reason: VelloNativeTextureValidationError::ResourceIdMismatch,
        } if rejected == texture
    ));
    assert!(operations.trace.is_empty());
    let lower = driver
        .replace(
            &scope,
            &registration,
            &base_resource,
            descriptor(2, 2),
            4,
            &mut operations,
        )
        .expect_err("lower replacement revision");
    assert!(matches!(
        lower,
        VelloPresenterError::NativeTextureRevisionRegressed { .. }
    ));
    assert!(operations.trace.is_empty());
    let equal = driver
        .replace(
            &scope,
            &registration,
            &base_resource,
            descriptor(2, 2),
            5,
            &mut operations,
        )
        .expect("equal replacement revision");
    assert_eq!(equal.texture_id(), texture);
    assert_eq!(operations.trace, vec!["invalidate", "override", "publish"]);
}

#[test]
fn missing_registration_returns_typed_error_without_mutation() {
    let texture = TextureId::from_raw(805);
    let scope = scope(8_005, 1);
    let registration = VelloNativeTextureRegistration {
        scope: scope.clone(),
        texture,
        registration_generation: 1,
    };
    let resource = resource(texture, 2.0, 2.0);
    let mut driver = NativeTextureMutationDriver::new();
    let mut operations = FakeOperations::default();
    let update = driver
        .update(&scope, &registration, 2, &mut operations)
        .expect_err("missing update");
    let replace = driver
        .replace(
            &scope,
            &registration,
            &resource,
            descriptor(2, 2),
            2,
            &mut operations,
        )
        .expect_err("missing replace");
    let remove = driver
        .remove(&scope, &registration, &mut operations)
        .expect_err("missing remove");
    assert!(matches!(
        update,
        VelloPresenterError::NativeTextureNotRegistered { .. }
    ));
    assert!(matches!(
        replace,
        VelloPresenterError::NativeTextureNotRegistered { .. }
    ));
    assert!(matches!(
        remove,
        VelloPresenterError::NativeTextureNotRegistered { .. }
    ));
    assert!(operations.trace.is_empty());
}

#[test]
fn generation_exhaustion_preserves_entry_and_backend_for_register_and_replace() {
    let texture = TextureId::from_raw(806);
    let scope = scope(8_006, 1);
    let resource = resource(texture, 2.0, 2.0);
    let mut driver = NativeTextureMutationDriver::new();
    driver.next_registration_generation = u64::MAX;
    let mut operations = FakeOperations::default();
    let register = driver
        .register(&scope, &resource, descriptor(2, 2), 1, &mut operations)
        .expect_err("register generation exhaustion");
    assert!(matches!(
        register,
        VelloPresenterError::NativeTextureGenerationExhausted
    ));
    assert!(operations.trace.is_empty());

    driver.next_registration_generation = 0;
    let registration = driver
        .register(&scope, &resource, descriptor(2, 2), 1, &mut operations)
        .expect("setup registration");
    operations.trace.clear();
    driver.next_registration_generation = u64::MAX;
    let replace = driver
        .replace(
            &scope,
            &registration,
            &resource,
            descriptor(2, 2),
            2,
            &mut operations,
        )
        .expect_err("replace generation exhaustion");
    assert!(matches!(
        replace,
        VelloPresenterError::NativeTextureGenerationExhausted
    ));
    assert!(operations.trace.is_empty());
    assert!(operations.active.contains_key(&texture));
}

#[test]
fn stale_cloned_token_cannot_mutate_reused_id() {
    let texture = TextureId::from_raw(807);
    let (scope, base_resource, mut driver, mut operations, registration) =
        registered_fixture(texture);
    let stale = registration.clone();
    driver
        .remove(&scope, &registration, &mut operations)
        .expect("remove original");
    let current = driver
        .register(&scope, &base_resource, descriptor(2, 2), 6, &mut operations)
        .expect("reuse texture ID");
    operations.trace.clear();
    let update = driver
        .update(&scope, &stale, 7, &mut operations)
        .expect_err("stale update");
    let replace = driver
        .replace(
            &scope,
            &stale,
            &base_resource,
            descriptor(2, 2),
            7,
            &mut operations,
        )
        .expect_err("stale replace");
    let remove = driver
        .remove(&scope, &stale, &mut operations)
        .expect_err("stale remove");
    assert!(matches!(
        update,
        VelloPresenterError::StaleNativeTextureRegistration { .. }
    ));
    assert!(matches!(
        replace,
        VelloPresenterError::StaleNativeTextureRegistration { .. }
    ));
    assert!(matches!(
        remove,
        VelloPresenterError::StaleNativeTextureRegistration { .. }
    ));
    assert_eq!(current.texture_id(), texture);
    assert!(operations.trace.is_empty());
    assert!(operations.active.contains_key(&texture));
}

#[test]
fn foreign_presenter_registration_is_rejected_before_mutation() {
    let texture = TextureId::from_raw(808);
    let (current_scope, resource, mut driver, mut operations, registration) =
        registered_fixture(texture);
    let foreign = scope(18_008, 1);
    operations.trace.clear();
    let update = driver
        .update(&foreign, &registration, 6, &mut operations)
        .expect_err("foreign update");
    let replace = driver
        .replace(
            &foreign,
            &registration,
            &resource,
            descriptor(2, 2),
            6,
            &mut operations,
        )
        .expect_err("foreign replace");
    let remove = driver
        .remove(&foreign, &registration, &mut operations)
        .expect_err("foreign remove");
    assert!(matches!(
        update,
        VelloPresenterError::StaleNativeTextureRegistration { .. }
    ));
    assert!(matches!(
        replace,
        VelloPresenterError::StaleNativeTextureRegistration { .. }
    ));
    assert!(matches!(
        remove,
        VelloPresenterError::StaleNativeTextureRegistration { .. }
    ));
    assert_eq!(registration.scope, current_scope);
    assert!(operations.trace.is_empty());
}

#[test]
fn panic_at_each_backend_step_leaves_no_stale_drawable_entry() {
    let texture = TextureId::from_raw(809);
    for (operation, expected) in [
        ("dirty", vec!["invalidate", "dirty"]),
        ("override", vec!["invalidate", "override"]),
        ("unregister", vec!["invalidate", "unregister"]),
        ("register", vec!["invalidate", "unregister", "register"]),
    ] {
        let (scope, base_resource, mut driver, mut operations, registration) =
            registered_fixture(texture);
        operations.trace.clear();
        operations.panic_on = Some(operation);
        let attempt = std::panic::catch_unwind(AssertUnwindSafe(|| match operation {
            "dirty" => {
                let _ = driver.update(&scope, &registration, 6, &mut operations);
            }
            "override" => {
                let _ = driver.replace(
                    &scope,
                    &registration,
                    &base_resource,
                    descriptor(2, 2),
                    6,
                    &mut operations,
                );
            }
            _ => {
                let resized = resource(texture, 3.0, 2.0);
                let _ = driver.replace(
                    &scope,
                    &registration,
                    &resized,
                    descriptor(3, 2),
                    6,
                    &mut operations,
                );
            }
        }));
        assert!(attempt.is_err());
        assert_eq!(operations.trace, expected);
        assert!(!operations.active.contains_key(&texture));
    }

    let (scope, _, mut driver, mut operations, registration) = registered_fixture(texture);
    operations.trace.clear();
    operations.panic_on = Some("unregister");
    let attempt = std::panic::catch_unwind(AssertUnwindSafe(|| {
        let _ = driver.remove(&scope, &registration, &mut operations);
    }));
    assert!(attempt.is_err());
    assert_eq!(operations.trace, vec!["invalidate", "unregister"]);
    assert!(!operations.active.contains_key(&texture));
}

#[test]
fn foreign_and_stale_device_scope_fail_before_native_mutation() {
    let (foreign, stale) = test_foreign_and_stale_native_scope_guards();
    assert!(matches!(
        foreign,
        Err(VelloPresenterError::ForeignPresenterScope)
    ));
    assert!(matches!(stale, Err(VelloPresenterError::StaleDeviceScope)));
}

#[test]
fn suspended_mutations_fail_without_losing_registration() {
    let (validated, current) = test_detached_native_scope_guards();
    assert!(matches!(
        validated,
        Err(VelloPresenterError::DeviceUnavailable)
    ));
    assert!(matches!(
        current,
        Err(VelloPresenterError::DeviceUnavailable)
    ));

    let texture = TextureId::from_raw(810);
    let (_, _, _, operations, registration) = registered_fixture(texture);
    assert_eq!(registration.texture_id(), texture);
    assert!(operations.active.contains_key(&texture));
}
