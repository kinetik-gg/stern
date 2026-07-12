# ADR 0001: GPU Presenter Contract

## Status

Accepted for the alpha implementation contract on 2026-07-12.

This decision closes `REND-ADR-01`; the reusable presenter and Showcase
adoption now close integrated `REND-03`. External-texture implementation remains
`REND-04`.

REND-03 is **Complete / Accepted**; REND-04 is **next**.

## Context

The renderer-neutral stack already translates `FrameOutput` into Vello scene
commands, but the only live device, surface, resize, submission, recovery, and
presentation path is private to the Showcase. `TextureId` is backend-neutral,
while `TextureResource` can carry an optional CPU snapshot; no supported public
path binds a domain-owned GPU resource to that ID.

The alpha path must let video, 3D, and image-processing renderers keep content
GPU-resident without putting Winit, wgpu, Vello, or operating-system objects in
`kinetik-ui-core` or `kinetik-ui-render`. It must also preserve the accepted
one-frame platform-request contract: presentation recovery may request another
frame, but it may not execute or replay shell work.

Pinned Vello 0.9 constrains the first implementation:

- `vello::util::RenderContext` owns the wgpu instance and compatible
  adapter/device/queue slots. Its surface factory returns a caller-owned
  `RenderSurface`, which owns the surface configuration, compatible device-slot
  ID, intermediate texture/view, and blitter.
- `vello::Renderer::register_texture` accepts an owned, cloneable
  `wgpu::Texture`, not an arbitrary `TextureView`. It is restricted to the
  registering Vello renderer.
- The documented input is `Rgba8Unorm` with `COPY_SRC` and straight alpha.
  Vello copies it on the GPU into its image atlas. Changed content requires an
  explicit dirty notification.
- This is a GPU-copy path with no CPU readback. It is not zero-copy, and Vello's
  documentation is not sufficient to promise that every unchanged repaint
  avoids an atlas copy.

## Decision

### Layer and crate boundary

The supported live integration will be implemented in a new concrete
`kinetik-ui-vello-winit` crate. It may depend on `kinetik-ui-vello`,
`kinetik-ui-winit`, Winit, Vello, and wgpu. No lower layer depends back on it.
We will not introduce a backend-neutral presenter trait until a second live
renderer backend demonstrates the common contract.

`kinetik-ui-vello::VelloRenderer` remains the scene translator/encoder. It does
not own windows, devices, queues, surfaces, or an event loop. The application
continues to own `ApplicationHandler`, UI/domain state, frame construction,
input normalization, shell request execution, and repaint scheduling.

| Authority | Owns | Must not own |
| --- | --- | --- |
| Application/event-loop runner | Winit lifecycle, UI and domain state, input, shell request execution, frame construction, repaint scheduling | GPU surface policy or renderer-native texture handles in UI state |
| Vello/Winit presenter | Presenter identity, device generation, `RenderContext`, current device/queue, explicit `Arc<Window>`, returned `RenderSurface`, Vello GPU renderer, acquire/render/blit/submit/present policy, external registry | Application commands, domain rendering logic, input routing, shell request execution |
| `VelloRenderer` scene backend | Backend-local scene translation, encoding, CPU image compatibility cache | Window, surface, device, queue, event loop |
| Domain renderer | Video/image/3D processing and producer resources created on the presenter's current device | UI placement, hit routing, overlays, presenter recovery |
| Core and neutral render crates | Stable IDs, primitives, metadata, CPU snapshots, diagnostics | Winit, wgpu, Vello, OS handles, native synchronization objects |
| `kinetik-ui-winit` | Event normalization and platform/shell/repaint adapters | Vello or GPU presentation |

### Presenter and device ownership

The alpha implementation supports exactly one live `VelloWindowPresenter`
bound to one native window. The presenter retains the authoritative
`Arc<Window>` used to create its surface and exposes that window's identity.
Event-loop code may retain a clone, but events for another `WindowId` do not
enter the presenter. This per-window ownership shape does not promise multiple
presenter instances during alpha.

A coordinator inside the presenter survives device rebuilds and mints an opaque
device scope containing both presenter identity and a monotonically increasing
generation. Domain renderers borrow the current `Device` and `Queue` through
that scope. GPU work displayed by the presenter is created on that device and
submitted to that queue.

wgpu does not expose the owning device from a public `Texture`. Therefore:

- presenter identity and generation are checked before registration or use;
- same-device provenance of a raw texture is a caller precondition; and
- a foreign-device texture is reported through the scoped wgpu validation
  failure path. The API does not falsely claim portable pre-validation.

Every attach or surface recreation compares the selected compatible device
slot with the current slot. An unchanged slot preserves the device generation.
A changed slot rebuilds the Vello GPU renderer, advances the generation, and
invalidates every external registration exactly as device loss does.

### Window and surface lifetime

The presenter owns surface creation, configuration, non-zero resize,
reconfiguration, recreation, intermediate target/view, acquire, Vello render,
blit submission, exactly one `Window::pre_present_notify`, and present.

Zero physical extent is a real non-presentable state. It is never configured as
a fabricated 1x1 surface. Restoring a non-zero extent configures the requested
size. Repeating the same extent is a no-op.

On suspend or detach, the presenter drops the surface state before releasing
its explicit window clone. Resume attaches a newly created window and surface;
application and input state remain outside the presenter. Redundant `Resumed`
while ready and redundant `Suspended` while detached are idempotent: they do
not create duplicate resources, leak state, or accept a stale window identity.

### External texture registry

The integration owns a backend-native registry keyed by stable `TextureId`.
The registry retains a cloned `wgpu::Texture`, the renderer-specific Vello
image handle, device scope, registration generation, content revision, extent,
format/alpha contract, and sampling metadata. Native objects never enter core
primitives, `UiMemory`, backend-neutral resource snapshots, serialization, or
semantics.

Texture resolution has one deterministic order:

1. a valid native registration for the current presenter/device/registration
   generation;
2. the existing compatible CPU snapshot in `TextureResource`; or
3. a visible placeholder plus one typed diagnostic.

A valid native registration suppresses `MissingTextureSnapshot`. A coordinated
helper keeps its extent and sampling metadata equal to the neutral
`TextureResource`; disagreement is an explicit registration/translation error,
not silent resampling. The CPU-snapshot path remains source-compatible.

Registration and updates use these rules:

- registering an active ID without explicit replace is an error;
- a changed content revision marks the Vello override dirty before scene
  rendering; an unchanged revision performs no toolkit-level dirty operation;
- same-extent replacement uses Vello's override operation and advances the
  registration generation;
- an extent change first invalidates lookup/draw resolution, unregisters the
  old Vello image, then registers a new image and generation;
- removal first invalidates lookup so no stale image can be encoded, then
  unregisters and drops the presenter's clone; and
- ID reuse is legal only after removal and receives a new registration
  generation.

These ordering rules prevent Vello's cross-renderer or post-unregister image
panic path. wgpu retains in-flight resource storage as required after logical
replace or removal.

### Format, color, and alpha

Vello directly documents `Rgba8Unorm`, `COPY_SRC`, same-renderer use, atlas
copy, and straight alpha. Kinetik deliberately narrows the alpha contract to a
full two-dimensional base-mip/base-layer texture with one sample and one mip.

RGB texel values carry the toolkit's sRGB-encoded numeric payload and alpha is
straight. `Rgba8Unorm` is the transport format; it does not authorize a
different color interpretation. Domain renderers perform HDR, wide-gamut, or
ICC conversion into that contract before registration. A `REND-04` live GPU
golden must verify the compositor's color and alpha result.

### Synchronization and presentation order

Alpha uses the presenter's one current queue and this order for each attempted
frame:

1. application/domain producer submission;
2. required native replace or dirty operation;
3. surface acquire;
4. scene encoding and Vello render-to-texture/atlas submission;
5. blit submission;
6. exactly one `Window::pre_present_notify`; and
7. surface presentation.

Queue submission order is the GPU synchronization contract. Dirty/replace
happens before scene rendering. A failed acquire performs no scene render, GPU
submission, pre-present notification, or present. Concurrent producer mutation
during presentation is unsupported.

No alpha API imports cross-device or cross-process memory, fences, semaphores,
or native shared handles.

### Failure and recovery

One `present` call makes at most one acquire attempt. It returns a typed outcome
and never drives the Winit loop or re-executes platform requests.

| Condition | Presenter behavior | Repaint guidance |
| --- | --- | --- |
| Non-zero success | Render, blit, notify once, present | Use the frame's normal request |
| Zero extent | Skip without configuring or acquiring | Wait for non-zero resize |
| Suboptimal acquire | Present the acquired frame, mark reconfigure-before-next | Request a later/next frame |
| Timeout | Skip before rendering or submission | Bounded/later retry through the scheduler |
| Occluded | Skip before rendering or submission | Wait for an external visibility/window event |
| Outdated | Reconfigure the existing surface | Return retry guidance; do not loop in the same call |
| Surface lost | Recreate/configure from the retained window, compare device slot | Return retry guidance; roll device generation if the slot changed |
| Validation failure | Return actionable integration failure | No blind immediate retry |
| Vello/render failure | Return a typed render failure | Caller decides whether to exit or rebuild |
| Device lost | Drop the old `RenderSurface`; rebuild the device/queue/Vello renderer; recreate/configure the surface from the retained window on the replacement context/device; rebuild target and blitter; advance generation; invalidate registry | Domain renderer recreates and re-registers before retry |
| Suspend/detach | Drop surface before presenter's window clone | Wait for resume/attach |

Surface retry guidance merges into repaint scheduling but never replays the
frame's shell request batch. Device recovery is observable: old scopes and all
native registrations fail deterministically until domain resources are
recreated on the new device and re-registered.

## Supported Alpha Contract

- Exactly one live presenter bound to one native window and surface.
- One current presenter device/queue and renderer per device generation.
- GPU-copy interoperability for same-device, full-base-subresource,
  `Rgba8Unorm + COPY_SRC`, straight-sRGB/straight-alpha textures.
- CPU snapshot compatibility fallback and visible missing-resource fallback.
- Non-zero resize, zero-size suspension, typed surface outcomes, surface
  recreation, and explicit whole-device generation recovery.
- Domain work submitted before presenter rendering on the same queue.
- Existing lower-level same-device Vello offscreen render-to-texture remains
  available independently of the window presenter.

## Explicitly Deferred or Unsupported

- Zero-copy/direct arbitrary `TextureView` sampling.
- Foreign-device, cross-process, shared-handle, fence, or semaphore import.
- `Rgba8UnormSrgb`, BGRA, premultiplied inputs, mip/view subsets, multisampling,
  HDR, wide gamut, and ICC handling inside the UI renderer.
- A reusable offscreen presenter abstraction. The existing lower-level Vello
  offscreen path is not a surface and receives no acquire/notify/present calls.
- Multiple independent presenter instances, general multi-window coordination,
  shared-device multi-surface policy, multiple adapters, and seamless migration
  between them.
- Additional presenter backends or a backend-neutral presenter trait.
- Production-grade transparent recovery that hides device loss from domain
  renderers.

## Consequences

The application gets one supported live composition path without contaminating
core or neutral snapshots. Domain resources can avoid CPU snapshots/readback,
but changing content may incur a GPU atlas copy and duplicate GPU memory. Large
video and 3D surfaces therefore retain a performance risk until measured.

The presenter is deliberately concrete and renderer-specific. This avoids an
unproved abstraction, but applications using a different window or renderer
backend need another integration later. Device loss is explicit and may be
visible to application code because domain resources must be recreated.

Alternatives rejected for alpha:

- application-owned injected device/queue, because first-path recovery and
  adapter-feature authority would be split;
- native handles in `TextureResource` or `RenderFrameInput`, because that would
  break backend neutrality and existing public struct contracts; and
- surface ownership inside `VelloRenderer`, because scene encoding would become
  coupled to Winit and live presentation.

## Follow-on Verification

`REND-03` is Complete / Accepted through the concrete presenter and public
Showcase adoption. Its evidence covers the pure lifecycle/result matrix,
zero/non-zero resize, redundant suspend/resume, window identity, changed
device-slot reattach, operation order, failed-acquire short circuit,
configuration propagation, diagnostics preservation, the runnable one-window
example, and the independent offscreen path's lack of surface acquire,
pre-present notification, or presentation.

`REND-04` is next and implements the native registry and Vello resolver without
changing the neutral public structs. It must prove register, replace, dirty/revision,
extent-change re-registration, remove, ID reuse, stale/cross-renderer/scope
rejection, device-generation invalidation, native/CPU/placeholder precedence,
valid-native diagnostic suppression, primitive order/clipping/overlays,
straight-sRGB/alpha composition, and a no-readback trace that forbids
texture-to-buffer copies, mapping, or CPU snapshot acquisition.

Any implementation that contradicts this ownership, lifetime, color,
synchronization, or recovery contract requires an explicit ADR amendment before
its implementation PR proceeds.
