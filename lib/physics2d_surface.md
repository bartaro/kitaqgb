# Fixed-Point Surface Contacts

These opt-in additions to `physics2d.h` / `physics2d.c` do not change the existing
AABB world, legacy gravity clamp, or legacy friction behavior.

## API

- `kq2d_scale_q8(value, coefficient)` multiplies a signed value by a signed Q8
  coefficient in [-256, 256], truncating toward zero. The endpoints represent
  exactly -1 and +1, unlike an s8 coefficient. An eight-step LR35902 multiply
  kernel applies the sign after truncation, preserving the previous arithmetic.
- `kq2d_body_limit_speed(body, max_speed)` applies one Q8 scale to both velocity
  components. A 65-entry hypot LUT plus bounded fractional division supplies a
  conservative circular speed limit without 32-bit products. Components and
  a positive limit must be at most 16383 in magnitude. Nonpositive limits stop
  the body. Small limits relative to huge input velocities can quantize to zero.
- `kq2d_body_resolve_surface(body, surface)` resolves the body's velocity against
  a kinematic surface. The caller selects active dynamic bodies and handles
  collision detection and penetration correction separately. It returns the
  incoming normal speed, or zero for a separating contact / null argument.
- `kq2d_surface_toi_q8(start_gap, end_gap)` returns the Q8 fraction of a linear
  outside-to-inside crossing. Positive gaps are outside. Initial overlap returns
  0; no crossing returns 256. Both gaps must fit +/-8191. This is a bounded
  crossing primitive, not a complete swept-shape or broad-phase implementation.

All velocities, bounce thresholds and powered kicks share the caller's unit.
Normals are unit vectors in Q8; conservative quantization is recommended.
Relative velocity components must fit +/-8191. Restitution and Coulomb friction
coefficients are unsigned Q0.8 (0..255). The tangential impulse is capped by the
normal impulse times friction, and never reverses slip. A kick is a minimum
outgoing normal speed, not an unlimited addition on every contact.

Below the bounce threshold the contact cancels approach without adding a bounce.
It reconstructs surface-relative tangent velocity instead of subtracting a
rounded normal impulse, reducing residual normal drift in resting contacts.
Small components (magnitude below 64 raw units) use round-to-nearest projection
with seven intermediate fractional bits. This prevents a shallow support from
discarding sub-unit tangential gravity on every update. The public scalar
multiply keeps its original truncation contract.
Surface-relative velocity makes an approaching moving flipper transfer energy,
while a held flipper has no motor kick. No position changes occur in this API.

The friction and low-speed restitution model follows conventional contact
impulses, described in the [Box2D simulation documentation](https://box2d.org/documentation/md_simulation.html).
This is not a Box2D port and does not include its full constraint solver, spin,
continuous collision detection or warm starting.

## Native Tests

From `C:\kitaqgb_project`:

```powershell
python pinball/probe_physics2d_surface.py
```

This compiles `pinball/test_physics2d_surface.c` with the production library and
runs its LR35902 machine code in KOKURA. The 8872 cases cover signs, cardinal and
oblique contacts, five material coefficient pairs, speed limits, low-speed rest,
moving surfaces, separating contacts, powered kicks, null arguments, a WANI
reflection reference and unchanged legacy gravity behavior. Scalar tests exercise
all 513 supported coefficients at 15 signed inputs, including representational
limits (the unrepresentable -32768 * -1 endpoint wraps in 16-bit arithmetic).
Crossing tests exercise 104 signed endpoint pairs, and 256 resting-contact
directions leave less than one raw unit of normal velocity (maximum 0.532).
Tests allow the
documented fixed-point quantization; no floating-point host substitute is used
for the implementation under test.

At the pinball's limit of 640 raw units, tested input headings deviate by at most
0.090 degrees. For tested input magnitudes below 2000, cap undershoot is at most
1.24%. These measured bounds are not a general guarantee for arbitrary tiny
limits or arbitrary normals. The largest measured passive speed increase from
rounding in the current contact suite is zero raw units. These bounds describe
the tested states, not every possible fixed-point input.
