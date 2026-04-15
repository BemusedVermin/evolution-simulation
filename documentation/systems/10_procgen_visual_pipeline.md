# Procedural Visual Pipeline: Genotype to CreatureBlueprint

## 1. Overview

The Procedural Visual Pipeline consumes **Visual Directives** from the interpreter (Stage 4) and produces a **CreatureBlueprint** — a complete, representation-agnostic description of a creature's form (skeleton, volumes, surfaces, materials, animations, effects). The blueprint is then consumed by renderer implementations (2D sprite, 3D mesh, or future formats) to produce final visuals.

**Core design principle**: The pipeline is **form-function coherent** — visual directives and ability emissions come from THE SAME interpreter rules. A rule that produces a "spike damage ability" simultaneously produces a "dorsal spikes visual directive." The creature that *bites* also *looks like a biter*. No designer can accidentally create a creature that looks harmless but deals massive damage.

The pipeline runs in six substages: (1) Skeleton Assembly, (2) Volume Shaping, (3) Surface Detailing, (4) Material Assignment, (5) Effect Attachment, (6) Animation Rigging. Each substage is pure function: same input directives → identical blueprint.

**Body-Site Channel Rendering (Issue #9)**: When phenotype interpreter emits per-body-site channel values (e.g., kinetic_force on limb_front_left vs. limb_front_right), the visual pipeline reads these directly and renders asymmetric geometry. Left-side limbs may appear thicker or sharper than right-side if their driving channels differ. Asymmetries emerge naturally from selection pressure on localized body regions.

---

## 2. Research Basis

**Procedural Animation Literature (Perlin, 2002; Sims, 1994)**: Skeletons as bone trees, driven by periodic functions or learned motion patterns. This doc uses deterministic skeleton topology derived from channel ratios (more elastic = more segments) rather than fixed body plans.

**Morphological Evolution (D'Arcy Thompson, "On Growth and Form")**: Body form emerges from underlying biological parameters (elasticity, rigidity, density). The pipeline operationalizes this: skeleton complexity, bone thickness, and volume shapes are direct functions of channels, not artist tweaks.

**Unreliable Narrator & Biome Mediation (Wolfe)**: Camouflage coloration is NOT hardcoded in the creature; it's a visual directive that says "apply dominant biome color." The environment interprets the creature's biology at render time.

**Layered Procedural Architecture (Procedural Content Generation, Togelius et al.)**: Each pipeline stage is independent and composable. A skeleton can be reused with different volume shapes. A material set can be applied to any skeleton.

---

## 3. Entities & State

### 3.1 Visual Directive (Input)

Produced by interpreter Stage 4:

```
VisualDirective {
    id: unique_id
    body_region: BodyRegion              // which part of the creature
    directive_type: enum {
        Protrude,    Harden,    Soften,  Orifice,   Append,    Inflate,   Texture,   Colorize
    }
    parameters: DirectiveParams          // type-specific
    priority: int                         // higher = more prominent
    source_channels: list<int>           // which channels drove this directive
}

DirectiveParams = union {
    // Protrude(shape, scale, density, distribution, surface_region)
    Protrude { 
        shape: enum { Spike, Horn, Plate, Knob, Hook, Tendril, Bulb },
        scale: float,
        density: int,                    // count per unit length
        distribution: enum { Regular, Random, Cluster },
        surface_region: enum { Dorsal, Ventral, Lateral, Anterior, AllSurface }
    },
    
    // Harden(roughness, segmentation) — add scales, plates, or roughness
    Harden {
        roughness: float,                // depth of texture
        segmentation: int,               // number of segments/plates
        pattern: enum { Scales, Plates, Ridges, Cracked }
    },
    
    // Soften(smoothness, transparency)
    Soften {
        smoothness: float,               // [0,1] how smooth
        transparency: float              // [0,1] for membranes
    },
    
    // Orifice(size, position, count)
    Orifice {
        size: float,
        position: float,                 // [0,1] along body length
        count: int,
        orientation: enum { Forward, Lateral, Ventral }
    },
    
    // Append(appendage_type, count, position)
    Append {
        appendage_type: enum { Fin, Wing, Tentacle, Horn, Crest },
        count: int,
        position: float                  // [0,1] along body
    },
    
    // Inflate(scale) — bloat a volume
    Inflate {
        scale: float                     // scale multiplier [0.8, 2.0]
    },
    
    // Texture(pattern, scale) — surface pattern
    Texture {
        pattern: enum { Scales, Bumps, Ridges, Mottled, Striped, Spotted, Pitted, Cracked, Smooth, Rocky, Wrinkled, Crystalline },
        scale: float                     // pattern frequency
    },
    
    // Colorize(base_color, emission, pattern)
    Colorize {
        base_color: ColorSpec,
        emission: ColorSpec or null,
        emission_intensity: float,
        pattern: TexturePattern or null,
        pattern_color_secondary: ColorSpec or null,
        contrast: float
    }
}

ColorSpec {
    hue: float                           // [0, 360] degrees
    saturation: float                    // [0, 1]
    value: float                         // [0, 1] brightness
    alpha: float                         // [0, 1] opacity
}
```

### 3.2 CreatureBlueprint (Output)

```
CreatureBlueprint {
    // Core structure
    skeleton: BoneTree
    volumes: list<Volume>
    surfaces: list<SurfaceDetail>
    materials: list<MaterialRegion>
    effects: list<AttachedEffect>
    animations: AnimationSet
    
    // Metadata for renderers
    metadata: BlueprintMetadata {
        bounding_box: AABB
        mass_center: Vec3                // for physics
        display_name: string
        life_stage: enum { Juvenile, Adult, Elderly }
        morphological_age: float         // [0,1] interpolation progress
    }
}

BoneTree {
    root: Bone
}

Bone {
    id: int
    name: string                         // auto-generated: "core_0", "limb_L_2"
    parent_id: int or null
    local_position: Vec3                 // relative to parent, z ≈ 0 for 2D-first
    local_rotation: float                // rest angle
    length: float
    thickness: float                     // base radius
    tags: set<BoneTag>
    constraints: JointConstraint
    children: list<Bone>
}

BoneTag = enum {
    Core, Head, Tail, Limb, LimbTip, Appendage, Jaw, Symmetric
}

JointConstraint {
    min_angle: float                     // in degrees
    max_angle: float
    stiffness: float                     // [0,1] resistance to rotation
    preferred: float                     // rest angle
}

Volume {
    id: int
    attached_bones: list<int>            // which bones this wraps
    shape: VolumeShape
    profile: CrossSectionProfile
    symmetry: SymmetryMode
    layer: int                           // for overlapping; higher = more outer
    damage_state: float                  // [0,1] how damaged (0 = pristine)
}

VolumeShape = enum {
    Ellipsoid { radii: Vec3 },
    Capsule { radius: float, length: float },
    Tapered { radius_start: float, radius_end: float },
    Bulbous { segments: list<float> },
    Custom { control_points: list<Vec2> }
}

CrossSectionProfile {
    // Defines how cross-section varies along bone length
    samples: list<float>                 // [0.5, 1.0, 1.0, 0.8, 0.3] = tapered at ends
}

SymmetryMode = enum {
    None,
    BilateralX,                          // left-right mirror
    BilateralY,                          // top-bottom mirror
    Radial { n: int }                    // n-fold rotational
}

SurfaceDetail {
    id: int
    target_volume: int                   // which volume this sits on
    detail_type: SurfaceType
    placement: Placement
    material_ref: int or null            // override material
}

SurfaceType = enum {
    Protrusion { shape, height, base_width, taper },
    Ridge { height, width, path },
    Pit { depth, radius },
    Texture { pattern, scale, depth },
    Orifice { radius, depth, rim_width },
    Membrane { span_bones, opacity, droop }
}

Placement {
    along_bone: float or Range           // [0,1] position on bone
    around_bone: float or Range          // [0,1] angle around cross-section
    spread: float                        // [0,1] coverage area
    count: int
    count_pattern: enum { Regular, Random, Cluster }
    mirror: bool                         // if volume is symmetric
}

MaterialRegion {
    id: int
    target: MaterialTarget               // volume, detail, or global
    properties: MaterialProps
}

MaterialTarget = enum {
    Volume { volume_id: int },
    Detail { detail_id: int },
    Global
}

MaterialProps {
    base_color: ColorSpec
    roughness: float                     // [0,1] 0=mirror, 1=chalk
    metallic: float                      // [0,1] 0=organic, 1=shiny
    subsurface: float                    // [0,1] translucence
    emission: ColorSpec or null          // glow color
    emission_power: float
    iridescence: float                   // color-shift intensity
    pattern: PatternOverlay or null      // layered color pattern
    wear_darkening: float                // how damaged is this [0,1]
}

PatternOverlay {
    pattern: enum { Scales, Bumps, Ridges, ... }
    color_a: ColorSpec
    color_b: ColorSpec
    scale: float
    contrast: float
}

AttachedEffect {
    id: int
    attach_point: AttachPoint
    effect_spec: EffectSpec
    active_when: EffectTrigger
}

AttachPoint = enum {
    BonePoint { bone_id: int, t: float },
    VolumeRegion { volume_id: int, placement: Placement },
    DetailPoint { detail_id: int },
    Aura { radius: float }
}

EffectSpec {
    type: enum { Particle, Drip, Smoke, Spore, Spark, Glow, Trail, Bubble, Ring },
    rate: float                          // per second
    color: ColorSpec
    size: float
    lifetime: float
    velocity: Vec3
    spread: float
    gravity: float
    fade: enum { None, Linear, Exponential }
}

EffectTrigger = enum {
    Always, WhenMoving, WhenAttacking, WhenDamaged, WhenInCombat, WhenIdle, Periodic { interval: float }
}

AnimationSet {
    locomotion: list<AnimationClip>      // walk, run, swim, slither
    idle: list<AnimationClip>            // breathing, pulsing
    attack: list<AnimationClip>          // per-ability attack animations
    damage: AnimationClip
    death: AnimationClip
    special: list<AnimationClip>         // ability-specific (cocoon, flash, etc.)
}

AnimationClip {
    name: string
    duration: float                      // seconds
    looping: bool
    bone_tracks: list<BoneTrack>
    effect_events: list<EffectEvent>     // when to trigger effects
    sound_events: list<SoundEvent>       // when to trigger sounds
}

BoneTrack {
    bone_id: int
    keyframes: list<Keyframe>
}

Keyframe {
    time: float                          // [0, duration]
    rotation: float                      // angle delta from rest
    scale: Vec2                          // squash/stretch
    easing: enum { Linear, EaseIn, EaseOut, EaseInOut, Bounce, Elastic }
}
```

---

## 4. Update Rules

### 4.1 Substage 1: Skeleton Assembly

Constructs the bone tree from the monster's channel profile.

```
function build_skeleton(
    resolved_phenotype: ResolvedPhenotype,
    visual_directives: list<VisualDirective>
) -> BoneTree:
    
    // 1. Infer body plan archetype from channels
    plan = infer_body_plan(resolved_phenotype)
    
    // 2. Build core spine (head + body segments + tail)
    core_count = plan.segment_count
    root = Bone(
        id: 0,
        name: "core_head",
        tags: { Core, Head },
        length: plan.head_length,
        thickness: plan.base_thickness,
        constraints: JointConstraint(
            min_angle: -15, max_angle: 15,
            stiffness: 0.8
        )
    )
    
    current = root
    for i in 1..core_count:
        tags = { Core }
        if i == core_count - 1: tags.add(Tail)
        
        child = Bone(
            id: i,
            name: "core_" + i,
            tags: tags,
            parent_id: current.id,
            length: plan.segment_length(i),
            thickness: plan.segment_thickness(i),
            local_position: Vec3(plan.segment_length(i-1), 0, 0),
            constraints: JointConstraint(
                min_angle: -plan.flexibility * 30,
                max_angle: plan.flexibility * 30,
                stiffness: 1.0 - plan.flexibility
            )
        )
        current.children.append(child)
        current = child
    
    // 3. Attach limbs (from body_map locomotion sites)
    limb_sites = find_limb_sites(resolved_phenotype.body_map)
    for site in limb_sites:
        limb = build_limb(site, resolved_phenotype)
        attach_to_nearest_core_bone(root, site.body_region, limb)
    
    // 4. Attach appendages (non-locomotion: fins, horns, antennae)
    for directive in visual_directives:
        if directive.directive_type == Append:
            appendage = build_appendage_from_directive(directive, resolved_phenotype)
            attach_to_body_region(root, directive.body_region, appendage)
    
    return BoneTree(root)

function infer_body_plan(phenotype: ResolvedPhenotype) -> BodyPlan:
    // Derive topology from channel ratios, not fixed categories
    
    elasticity = phenotype.global_channels[ELASTIC_DEFORMATION]
    rigidity = phenotype.global_channels[STRUCTURAL_RIGIDITY]
    mass = phenotype.global_channels[MASS_DENSITY]
    speed = phenotype.global_channels[METABOLIC_RATE]
    friction = phenotype.global_channels[SURFACE_FRICTION]
    
    // Core segment count: elastic creatures are wormy, rigid creatures are few-segmented
    segment_count = clamp(
        round(3 + elasticity * 5 - rigidity * 3),
        2,  // minimum: head + tail
        8   // maximum
    )
    
    // Base thickness: heavier = thicker bones
    base_thickness = 0.3 + mass * 0.7
    
    // Spine flexibility: how much segments bend
    flexibility = clamp(elasticity * 0.8 - rigidity * 0.5, 0.05, 0.95)
    
    // Limb count: from locomotion channels
    // High friction + low elasticity = legs (grip ground)
    // High elasticity + low friction = tentacles
    // High kinetic + low mass = wings
    limb_potential = (
        friction * 0.4 +
        (1.0 - elasticity) * 0.3 +
        speed * 0.3
    )
    limb_count = clamp(round(limb_potential * 6), 0, 8)
    
    // Enforce bilateral symmetry (most common)
    if limb_count % 2 != 0 and random() > 0.15:
        limb_count += 1
    
    return BodyPlan {
        segment_count,
        base_thickness,
        flexibility,
        limb_count,
        head_length: base_thickness * 1.5,
        segment_length: fn(i) => base_thickness * (2.0 - i * 0.1),
        segment_thickness: fn(i) => base_thickness * (1.0 - i * 0.05)
    }

function build_limb(site: LimbSite, phenotype: ResolvedPhenotype) -> Bone:
    kinetic = phenotype.global_channels[KINETIC_FORCE]
    metabolic = phenotype.global_channels[METABOLIC_RATE]
    rigidity = phenotype.global_channels[STRUCTURAL_RIGIDITY]
    
    // Limb length: driven by kinetic force and speed
    length = 0.3 + kinetic * 0.5 + metabolic * 0.3
    
    // Number of joints in limb (1 = stub, 2 = normal, 3 = articulated)
    joint_count = 1
    if length > 0.5: joint_count = 2
    if length > 0.8: joint_count = 3
    
    limb_root = Bone(
        id: alloc_id(),
        name: site.name,  // "limb_L_0"
        tags: { Limb, Symmetric },
        length: length / joint_count,
        thickness: 0.15 + rigidity * 0.2,
        constraints: JointConstraint(
            min_angle: -60, max_angle: 60,
            stiffness: 0.3 + rigidity * 0.5
        )
    )
    
    current = limb_root
    for j in 1..joint_count:
        segment = Bone(
            id: alloc_id(),
            tags: if j == joint_count - 1 then { LimbTip } else { Limb },
            name: site.name + "_" + j,
            parent_id: current.id,
            length: (length / joint_count) * (1.0 - j * 0.15),  // taper
            thickness: current.thickness * 0.7,
            constraints: JointConstraint(
                min_angle: -45, max_angle: 45,
                stiffness: current.constraints.stiffness * 0.8
            )
        )
        current.children.append(segment)
        current = segment
    
    return limb_root
```

### 4.2 Substage 2: Volume Shaping

Wraps implicit volumes around the skeleton.

```
function shape_volumes(
    skeleton: BoneTree,
    resolved_phenotype: ResolvedPhenotype,
    visual_directives: list<VisualDirective>
) -> list<Volume>:
    
    volumes = []
    
    for bone in skeleton.all_bones():
        shape = choose_volume_shape(bone, resolved_phenotype)
        profile = build_cross_section_profile(bone, resolved_phenotype)
        
        vol = Volume(
            id: alloc_id(),
            attached_bones: [bone.id],
            shape: shape,
            profile: profile,
            symmetry: if Symmetric in bone.tags then BilateralX else None,
            layer: 0,
            damage_state: 0.0
        )
        volumes.append(vol)
    
    // Merge adjacent core volumes if rigid (shell-like body)
    if resolved_phenotype.global_channels[STRUCTURAL_RIGIDITY] > 0.6:
        volumes = merge_rigid_volumes(volumes, skeleton)
    
    // Apply Inflate directives
    for directive in visual_directives:
        if directive.directive_type == Inflate:
            target_vols = match_directive_to_volumes(directive, volumes, skeleton)
            for vol_id in target_vols:
                volumes[vol_id].shape.scale_radii(directive.parameters.scale)
    
    return volumes

function choose_volume_shape(bone: Bone, phenotype: ResolvedPhenotype) -> VolumeShape:
    elasticity = phenotype.global_channels[ELASTIC_DEFORMATION]
    rigidity = phenotype.global_channels[STRUCTURAL_RIGIDITY]
    
    if Core in bone.tags:
        if elasticity > 0.5:
            // Smooth, bulbous core
            return Bulbous(
                segments: smooth_bulge_curve(bone.length, peak_at: 0.5)
            )
        elif rigidity > 0.5:
            // Rigid, armored core
            return Capsule(
                radius: bone.thickness * 1.2,
                length: bone.length
            )
        else:
            // Default: slightly tapered
            return Tapered(
                radius_start: bone.thickness,
                radius_end: bone.thickness * 0.8
            )
    
    elif Limb in bone.tags:
        // Limbs are always tapered, thinner at tips
        return Tapered(
            radius_start: bone.thickness,
            radius_end: bone.thickness * 0.5
        )
    
    elif Appendage in bone.tags:
        // Appendages (fins, tentacles) taper to points
        return Tapered(
            radius_start: bone.thickness * 0.8,
            radius_end: bone.thickness * 0.1
        )
    
    else:
        return Ellipsoid(radii: Vec3(bone.thickness, bone.thickness, bone.thickness))

function build_cross_section_profile(bone: Bone, phenotype: ResolvedPhenotype) -> CrossSectionProfile:
    // How does the cross-section vary along the bone's length?
    
    elasticity = phenotype.global_channels[ELASTIC_DEFORMATION]
    rigidity = phenotype.global_channels[STRUCTURAL_RIGIDITY]
    
    // Elastic creatures have smooth profiles (sine curves)
    // Rigid creatures have segmented profiles (plateau then taper)
    
    samples = []
    for t in [0.0, 0.25, 0.5, 0.75, 1.0]:
        if elasticity > 0.6:
            // Smooth: use sine
            scale = 0.5 + 0.5 * sin(t * PI)
        else:
            // Segmented: plateau in middle, taper at ends
            if t < 0.2 or t > 0.8:
                scale = 0.6 + t * 0.2  // ramp up, then ramp down
            else:
                scale = 1.0
        
        samples.append(scale)
    
    return CrossSectionProfile(samples)
```

### 4.3 Substage 3: Surface Detailing

Applies surface modifications from visual directives.

```
function apply_surface_details(
    volumes: list<Volume>,
    skeleton: BoneTree,
    visual_directives: list<VisualDirective>
) -> list<SurfaceDetail>:
    
    details = []
    
    for directive in visual_directives:
        if directive.directive_type in [Protrude, Harden, Orifice, Texture]:
            // Find which volume(s) this targets
            target_volumes = match_directive_to_volumes(directive, volumes, skeleton)
            
            for vol_id in target_volumes:
                detail = translate_directive_to_surface_detail(
                    directive, vol_id, volumes[vol_id]
                )
                if detail != null:
                    details.append(detail)
    
    return details

function translate_directive_to_surface_detail(
    directive: VisualDirective,
    target_vol_id: int,
    target_vol: Volume
) -> SurfaceDetail or null:
    
    match directive.directive_type:
        Protrude:
            params = directive.parameters
            return SurfaceDetail(
                id: alloc_id(),
                target_volume: target_vol_id,
                detail_type: Protrusion(
                    shape: map_shape_name(params.shape),
                    height: params.scale,
                    base_width: params.scale * 0.3,
                    taper: if params.shape in [Spike, Horn] then 0.9 else 0.3
                ),
                placement: Placement(
                    along_bone: 0.0..1.0,
                    around_bone: infer_surface_from_region(directive.body_region),
                    spread: params.density / 10.0,  // normalize
                    count: params.density,
                    count_pattern: params.distribution,
                    mirror: target_vol.symmetry != None
                )
            )
        
        Harden:
            params = directive.parameters
            return SurfaceDetail(
                id: alloc_id(),
                target_volume: target_vol_id,
                detail_type: Texture(
                    pattern: params.pattern,
                    scale: 1.0 / params.segmentation,
                    depth: params.roughness * 0.05
                ),
                placement: Placement(
                    spread: 1.0,     // covers entire volume
                    count: 1,
                    mirror: target_vol.symmetry != None
                )
            )
        
        Orifice:
            params = directive.parameters
            return SurfaceDetail(
                id: alloc_id(),
                target_volume: target_vol_id,
                detail_type: Orifice(
                    radius: params.size,
                    depth: params.size * 0.5,
                    rim_width: params.size * 0.2
                ),
                placement: Placement(
                    along_bone: params.position,
                    count: params.count,
                    mirror: false
                )
            )
        
        Texture:
            params = directive.parameters
            return SurfaceDetail(
                id: alloc_id(),
                target_volume: target_vol_id,
                detail_type: Texture(
                    pattern: params.pattern,
                    scale: params.scale,
                    depth: 0.02
                ),
                placement: Placement(
                    spread: 1.0,
                    count: 1,
                    mirror: target_vol.symmetry != None
                )
            )
        
        _: return null
```

### 4.4 Substage 4: Material Assignment

Maps color and material properties from directives.

```
function assign_materials(
    volumes: list<Volume>,
    surfaces: list<SurfaceDetail>,
    visual_directives: list<VisualDirective>,
    biome: BiomeID
) -> list<MaterialRegion>:
    
    regions = []
    
    // Global base material
    global_mat = compute_base_material(visual_directives)
    regions.append(MaterialRegion(
        id: alloc_id(),
        target: Global,
        properties: global_mat
    ))
    
    // Per-volume overrides from Colorize directives
    for directive in visual_directives:
        if directive.directive_type == Colorize:
            target_vols = match_directive_to_volumes(directive, volumes, ...)
            
            for vol_id in target_vols:
                params = directive.parameters
                
                // Resolve environment-dependent colors
                base_color = params.base_color
                if base_color.hue == -1:  // sentinel: use biome color
                    base_color = biome.dominant_color
                
                regions.append(MaterialRegion(
                    id: alloc_id(),
                    target: Volume { volume_id: vol_id },
                    properties: MaterialProps(
                        base_color: base_color,
                        roughness: 0.2,
                        emission: params.emission,
                        emission_power: params.emission_intensity,
                        pattern: PatternOverlay(
                            pattern: params.pattern,
                            color_a: params.pattern_color_secondary or base_color,
                            scale: params.scale or 0.3,
                            contrast: params.contrast or 0.5
                        ) if params.pattern != null else null
                    )
                ))
    
    // Per-detail material overrides
    for detail in surfaces:
        if detail.material_ref != null:
            regions.append(MaterialRegion(
                id: detail.material_ref,
                target: Detail { detail_id: detail.id },
                properties: /* ... */
            ))
    
    return regions

function compute_base_material(directives: list<VisualDirective>) -> MaterialProps:
    // Infer base material from channel profile
    // (simplified: could be much richer)
    
    return MaterialProps(
        base_color: ColorSpec(hue: 30, saturation: 0.5, value: 0.6),
        roughness: 0.3,
        metallic: 0.0,
        subsurface: 0.1
    )
```

### 4.5 Substage 5: Effect Attachment

Attaches particle effects and glows.

```
function attach_effects(
    skeleton: BoneTree,
    volumes: list<Volume>,
    surfaces: list<SurfaceDetail>,
    resolved_phenotype: ResolvedPhenotype
) -> list<AttachedEffect>:
    
    effects = []
    
    // Example: Bioluminescent glow (from LIGHT_EMISSION channel)
    if resolved_phenotype.global_channels[LIGHT_EMISSION] > 0.3:
        emission_intensity = resolved_phenotype.global_channels[LIGHT_EMISSION]
        
        effects.append(AttachedEffect(
            id: alloc_id(),
            attach_point: Aura { radius: emission_intensity * 2 },
            effect_spec: EffectSpec(
                type: Glow,
                color: hue_from_channel(LIGHT_EMISSION),
                emission_power: emission_intensity * 3,
                rate: 0  // static glow
            ),
            active_when: Always
        ))
    
    // Example: Dripping venom (from CHEMICAL_OUTPUT channel)
    if resolved_phenotype.global_channels[CHEMICAL_OUTPUT] > 0.4:
        chemical_intensity = resolved_phenotype.global_channels[CHEMICAL_OUTPUT]
        
        // Find volumes with high chemical concentration
        for vol in volumes:
            if vol.attached_bones.contains(find_bone_by_tag(skeleton, Jaw)):
                effects.append(AttachedEffect(
                    id: alloc_id(),
                    attach_point: VolumeRegion {
                        volume_id: vol.id,
                        placement: Placement(around_bone: 0.0)  // front/underside
                    },
                    effect_spec: EffectSpec(
                        type: Drip,
                        color: ColorSpec(hue: 120, saturation: 0.8, value: 0.6),
                        rate: chemical_intensity * 5,
                        lifetime: 2.0,
                        gravity: 1.0,
                        fade: Linear
                    ),
                    active_when: Always
                ))
    
    // Example: Thermal shimmer (from THERMAL_OUTPUT)
    if resolved_phenotype.global_channels[THERMAL_OUTPUT] > 0.5:
        thermal_intensity = resolved_phenotype.global_channels[THERMAL_OUTPUT]
        
        effects.append(AttachedEffect(
            id: alloc_id(),
            attach_point: Aura { radius: 1.0 },
            effect_spec: EffectSpec(
                type: Particle,
                color: ColorSpec(hue: 0, saturation: 1.0, value: 1.0),  // red-orange
                rate: thermal_intensity * 10,
                size: 0.1,
                lifetime: 0.5,
                gravity: -0.5,  // rise
                fade: Exponential
            ),
            active_when: WhenMoving
        ))
    
    return effects
```

### 4.6 Substage 6: Animation Rigging

Generates animations from skeleton topology and channel-derived movement styles.

```
function rig_animations(
    skeleton: BoneTree,
    resolved_phenotype: ResolvedPhenotype
) -> AnimationSet:
    
    anim_set = new AnimationSet()
    
    // Locomotion animations depend on skeleton topology and elasticity
    elasticity = resolved_phenotype.global_channels[ELASTIC_DEFORMATION]
    rigidity = resolved_phenotype.global_channels[STRUCTURAL_RIGIDITY]
    metabolic_rate = resolved_phenotype.global_channels[METABOLIC_RATE]
    
    // Infer movement style from channels
    if elasticity > 0.6:
        // Sinuous, undulating movement (worms, snakes)
        locomotion_style = SinuousWave(amplitude: elasticity * 0.5, frequency: metabolic_rate * 2)
    elif rigidity > 0.6:
        // Segmented, stiff movement (armored creatures)
        locomotion_style = SegmentedScuttle(segment_lag: 0.1)
    else:
        // Limb-based movement (quadrupeds, bipeds)
        limb_count = skeleton.count_limbs()
        if limb_count >= 4:
            locomotion_style = QuadrupedWalk(stride_length: 0.3)
        else:
            locomotion_style = BipedWalk(stride_length: 0.4)
    
    // Generate walk/run animations
    anim_set.locomotion.append(generate_locomotion_animation(
        skeleton,
        locomotion_style,
        speed: 1.0,
        duration: 2.0
    ))
    
    anim_set.locomotion.append(generate_locomotion_animation(
        skeleton,
        locomotion_style,
        speed: 2.0,
        duration: 1.0
    ))
    
    // Idle animation (breathing, pulsing)
    anim_set.idle.append(generate_idle_animation(skeleton, elasticity))
    
    // Damage reaction
    anim_set.damage = generate_damage_animation(skeleton)
    
    // Death
    anim_set.death = generate_death_animation(skeleton)
    
    return anim_set

function generate_locomotion_animation(
    skeleton: BoneTree,
    movement_style: MovementStyle,
    speed: float,
    duration: float
) -> AnimationClip:
    
    clip = AnimationClip(
        name: "walk_speed_" + speed,
        duration: duration,
        looping: true
    )
    
    match movement_style:
        SinuousWave(amplitude, frequency):
            // Generate sinuous spine undulation
            for bone in skeleton.core_bones():
                keyframes = []
                for t in 0.0..duration step 0.05:
                    phase = t * frequency
                    rotation = sin(phase + bone.id * PI/8) * amplitude * 45  // degrees
                    keyframes.append(Keyframe(
                        time: t,
                        rotation: rotation,
                        easing: Linear
                    ))
                clip.bone_tracks.append(BoneTrack(bone_id: bone.id, keyframes))
        
        QuadrupedWalk(stride_length):
            // Generate opposing limb movement
            limbs = skeleton.find_limbs()
            for (i, limb) in limbs.enumerate():
                keyframes = []
                phase_offset = if (i % 2 == 0) then 0.0 else 0.5  // alternating sides
                
                for t in 0.0..duration step 0.05:
                    phase = (t + phase_offset) % 1.0
                    
                    // Lift at 25%, extend at 50%, plant at 75%, recover at 100%
                    if phase < 0.25:
                        rotation = (phase / 0.25) * 30  // lift up
                    elif phase < 0.75:
                        rotation = 30 - ((phase - 0.25) / 0.5) * 60  // swing forward
                    else:
                        rotation = -30 + ((phase - 0.75) / 0.25) * 30  // plant and recover
                    
                    keyframes.append(Keyframe(
                        time: t,
                        rotation: rotation,
                        easing: EaseInOut
                    ))
                
                clip.bone_tracks.append(BoneTrack(bone_id: limb.id, keyframes))
    
    return clip
```

---

## 5. Cross-System Hooks

1. **Interpreter (Stage 4)**: Reads visual directives emitted by the unified expression rules.

2. **Combat System**: Collision volumes derived from blueprint volumes determine hit detection. Damaged volumes reduce material yield.

3. **Renderer** (2D/3D): Consumes CreatureBlueprint and produces final visual asset. Both renderers consume the same blueprint format.

4. **Crafting System**: Harvested materials come from volumes' channel profiles. Damaged volumes yield less.

5. **Physics**: Mass center, bounding box, and skeleton structure feed into movement and collision.

6. **Age/Life Cycle**: `BlueprintMetadata.morphological_state` interpolates skeleton poses as creatures age (juvenile→adult→elderly).

---

## 6. Tradeoff Matrix

| Dimension | Channel-Driven | Hand-Authored | Winner |
|-----------|---|---|---|
| **Coherence** | Skeleton complexity = channel ratio (no mismatches) | Artist can decouple form from function | Channel-Driven |
| **Realism** | Biological plausibility (elastic=wavy, rigid=stiff) | Can override for style | Channel-Driven (simulation) |
| **Control** | Limited — topology emergent from channels | Full control | Hand-Authored (short-term) |
| **Scalability** | Add new channel → all creatures automatically vary | Must hand-tune each creature | Channel-Driven |
| **Variation** | Thousands of distinct morphologies from 20 channels | Manual authoring | Channel-Driven |
| **Iteration** | Change one channel weight → whole population shifts | Hand-edit each variant | Channel-Driven (design velocity) |

**Winner**: Channel-Driven. Simulation coherence and scale outweigh artistic control.

---

## 7. Emergent Properties

1. **Morphological diversity**: From ELASTIC_DEFORMATION + STRUCTURAL_RIGIDITY + MASS_DENSITY alone, skeletons range from worm-like (elastic) to turtle-like (rigid) to dragon-like (balanced).

2. **Form-function coherence**: A creature with high KINETIC_FORCE also has:
   - Longer limbs (from limb construction rules)
   - Predatory jaw structure (from visual directives)
   - Melee damage ability (from assembly rules)
   These are not hard-coded separately — they emerge from the same channel pattern.

3. **Age-based morphological progression**: Juvenile form → Adult form uses the same skeleton topology but with scaled volumes and different animation parameters. Elderly creatures show wear (damaged volumes, darkened materials).

4. **Damage visualization**: When a creature takes damage to a limb, the corresponding volume's `damage_state` increases. This affects material appearance (darkening, scarring) and material yield on harvest.

5. **Biome-adaptive appearance**: A creature with high LIGHT_ABSORPTION in a dark forest will be rendered dark (matching biome). The same genotype in a bright desert will be rendered lighter. The genotype doesn't change — the visual directive's interpretation does.

---

## 8. Open Calibration Knobs

1. **Segment count formula**: `3 + elasticity * 5 - rigidity * 3`. Adjust multipliers to make elastic creatures wormier or stiffer creatures fewer-segmented.

2. **Limb count formula**: `(friction * 0.4 + (1 - elasticity) * 0.3 + speed * 0.3) * 6`. Reweight channels to favor legs vs. tentacles vs. fins.

3. **Volume profile sampling**: How many cross-section samples? More = smoother bodies. Currently 5 samples.

4. **Surface detail density**: Protrude directive density scales from channels. Tweak the scaling function to make spikes denser/sparser.

5. **Animation frequency**: Locomotion animation frequency scales with METABOLIC_RATE. Adjust multiplier to make fast creatures frantic or stately.

6. **Effect emission rates**: Thermal shimmer rate = `thermal_intensity * 10`. Adjust to make effects more/less visible.

7. **Morphological lerp speed**: When creatures age or enter new life stages, how fast does the skeleton interpolate? Fast = abrupt growth; slow = gradual.

---

## 9. Morphogenesis & Body Damage

### Morphological State Tracking

```
MorphState {
    current_form: SkeletonSnapshot
    target_form: SkeletonSnapshot
    progress: float                      // [0, 1] lerp parameter
    morphogenesis_ticks: uint64          // how many ticks until target reached
    life_stage: enum { Juvenile, Adult, Elderly }
}
```

When a monster ages from Juvenile to Adult:
- `target_form` is the adult skeleton (larger, different proportions)
- `progress` starts at 0, increments each tick
- Each rendered frame, skeleton is interpolated: `lerp(current, target, progress)`
- At `progress == 1`, `current_form = target_form`, new adult abilities unlock

### Damage Accumulation

Each volume tracks accumulated damage:
```
Volume {
    ...
    damage_state: float                  // [0, 1]
}
```

When creature takes damage to a limb:
- The corresponding volume's `damage_state` increases
- This affects:
  - Material appearance: materials with `wear_darkening` become darker
  - Material yield: harvested amount = `base_yield * (1 - damage_state)`
  - If damage_state > 0.8, volume may become visually "broken" (cracked, missing pieces)

---

## 10. Implementation Notes

- **Determinism**: All skeleton, volume, and animation generation is deterministic. Hash(genotype) uniquely identifies the blueprint. Renderers can cache blueprints per genotype.

- **Renderer Independence**: Skeleton and volumes are 2D-compatible (z ≈ 0). Both sprite and mesh renderers can consume the same blueprint.

- **Performance**: Skeleton assembly is O(bone_count), typically 10-30 bones. Volume shaping is O(bones). Surface detailing is O(directives), typically 5-15 per creature.

