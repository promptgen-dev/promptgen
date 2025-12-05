//! Shared test utilities and the kitchen sink test library.
//!
//! This module provides helper functions for evaluating templates in tests,
//! along with a comprehensive test library that exercises all grammar features.

#![allow(dead_code)]

use once_cell::sync::Lazy;
use promptgen_core::{
    parse_pack, parse_template, render, EvalContext, Library, PromptTemplate, RenderError,
    RenderResult,
};

/// A comprehensive test library that includes:
/// - Multiple groups with various tag configurations
/// - Weighted options
/// - Groups with multiple tags (aliases)
/// - Groups designed for exclusion testing
pub const KITCHEN_SINK_YAML: &str = r#"
id: kitchen-sink
name: Kitchen Sink Test Library
description: A comprehensive library for testing all grammar features

groups:
  # Basic appearance groups
  - tags: [Hair, hair-color, appearance, Hair Color]
    options:
      - text: blonde hair
      - text: red hair
      - text: black hair
      - text: brown hair

  - tags: [Eyes, eye-color, appearance]
    options:
      - text: blue eyes
      - text: green eyes
      - text: brown eyes
      - text: heterochromia

  - tags: [Outfit, clothing]
    options:
      - text: casual dress
      - text: formal suit
      - text: school uniform
      - text: fantasy armor

  # Style groups
  - tags: [Style, art-style]
    options:
      - text: photorealistic
      - text: anime style
      - text: oil painting
      - text: watercolor

  # Groups for exclusion testing - tagged with substyles
  - tags: [AnimeEyes, Eyes, anime]
    options:
      - text: large sparkling eyes
      - text: chibi eyes

  - tags: [RealisticEyes, Eyes, realistic]
    options:
      - text: detailed iris
      - text: natural eye shape

  # Quality and technical tags
  - tags: [Quality]
    options:
      - text: masterpiece, best quality
      - text: high quality
      - text: normal quality

  - tags: [Lighting]
    options:
      - text: soft lighting
      - text: dramatic lighting
      - text: natural sunlight
      - text: studio lighting

  # Character traits
  - tags: [Expression, emotion]
    options:
      - text: smiling
      - text: serious expression
      - text: surprised look
      - text: gentle smile

  - tags: [Pose]
    options:
      - text: standing
      - text: sitting
      - text: walking
      - text: dynamic pose

  # Scene elements
  - tags: [Background, scene]
    options:
      - text: simple background
      - text: outdoors, nature
      - text: indoor, room
      - text: fantasy landscape

  - tags: [TimeOfDay, lighting-natural]
    options:
      - text: daytime
      - text: sunset
      - text: night sky
      - text: golden hour

templates:
  - id: basic-character
    name: Basic Character
    description: A simple character template
    tags: [character, simple]
    source: "{Quality}, {Hair}, {Eyes}, {Expression}"

  - id: full-character
    name: Full Character
    description: A comprehensive character template with all features
    tags: [character, detailed]
    source: |
      {Quality}, {Style}
      {Hair}, {Eyes}, {Expression}
      {Outfit}, {Pose}
      {Background}, {Lighting}

  - id: freeform-scene
    name: Freeform Scene
    description: Template with freeform slots for custom input
    tags: [scene, custom]
    source: "{Quality}, {{ Subject }}, {{ Action }}, {Background}, {Lighting}"

  - id: eyes-exclusion
    name: Eyes with Exclusion
    description: Tests tag exclusion syntax
    tags: [test, exclusion]
    source: "{Eyes - anime}"

  - id: expression-block
    name: Expression Block Test
    description: Tests expression block with pipeline
    tags: [test, expression]
    source: "[[ \"Hair\" | some | assign(\"chosen_hair\") ]], {Eyes}"
"#;

/// Lazily loaded test library - parsed once and reused across all tests.
static TEST_LIBRARY: Lazy<Library> =
    Lazy::new(|| parse_pack(KITCHEN_SINK_YAML).expect("Kitchen sink YAML should be valid"));

/// Get a reference to the shared test library.
pub fn load_test_library() -> &'static Library {
    &TEST_LIBRARY
}

/// Evaluate a template source string against the test library.
///
/// # Arguments
/// * `source` - The template source string to evaluate
/// * `seed` - Optional seed for deterministic results. If None, uses seed 42.
///
/// # Returns
/// The rendered result containing the output text and metadata.
///
/// # Panics
/// Panics if the template fails to parse or render.
pub fn eval(source: &str, seed: Option<u64>) -> RenderResult {
    eval_with_slots(source, &[], seed)
}

/// Evaluate a template with slot overrides.
///
/// # Arguments
/// * `source` - The template source string to evaluate
/// * `slots` - Slice of (name, value) pairs for slot overrides
/// * `seed` - Optional seed for deterministic results. If None, uses seed 42.
pub fn eval_with_slots(source: &str, slots: &[(&str, &str)], seed: Option<u64>) -> RenderResult {
    let lib = load_test_library();
    let ast = parse_template(source).expect("Template should parse");
    let template = PromptTemplate::new("test", ast);
    let mut ctx = EvalContext::with_seed(lib, seed.unwrap_or(42));
    for (name, value) in slots {
        ctx.set_slot(*name, *value);
    }
    render(&template, &mut ctx).expect("Template should render")
}

/// Evaluate a pre-defined template from the library by name.
///
/// # Arguments
/// * `name` - The name of the template in the library
/// * `slots` - Slice of (name, value) pairs for slot overrides
/// * `seed` - Optional seed for deterministic results. If None, uses seed 42.
pub fn eval_template(name: &str, slots: &[(&str, &str)], seed: Option<u64>) -> RenderResult {
    let lib = load_test_library();
    let template = lib
        .find_template(name)
        .unwrap_or_else(|| panic!("Template '{}' should exist", name));
    let mut ctx = EvalContext::with_seed(lib, seed.unwrap_or(42));
    for (slot_name, value) in slots {
        ctx.set_slot(*slot_name, *value);
    }
    render(template, &mut ctx).expect("Template should render")
}

/// Try to evaluate a template, returning the Result for error testing.
pub fn try_eval(source: &str, seed: Option<u64>) -> Result<RenderResult, RenderError> {
    let lib = load_test_library();
    let ast = parse_template(source).expect("Template should parse");
    let template = PromptTemplate::new("test", ast);
    let mut ctx = EvalContext::with_seed(lib, seed.unwrap_or(42));
    render(&template, &mut ctx)
}
