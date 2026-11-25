# Selective projectdetect reuse

Source: [projectdetect v0.2.0 at 6b75606](https://github.com/richardwooding/projectdetect-rs/tree/6b756065e0cb74a2e121381afa7cead4c145b262), MIT license, copyright 2026 Richard Wooding. Retain license and NOTICE.

The built-in project-type list in `src/builtins.rs` provides candidate positive filesystem markers, not a new detection engine. Add 18 non-overlapping technology definitions and 42 rules in `tests/fixtures/nosey/projectdetect-registry`; extend the existing what-stack .NET candidate with 5 new rules rather than minting a duplicate .NET technology ID. All IDs are fixture-only, no skill mappings or production activation.

## Translation boundaries

- Overlapping Go, Rust, Maven, Gradle, .NET, PHP, Astro and Gatsby definitions remain in the what-stack fixture set. F# remains separately defined there. The five new .NET indicators are Directory.Build.props, Directory.Packages.props, nuget.config, .vbproj and .slnf. Omit ambiguous global.json.
- Case-insensitive exact basename regexes mirror projectdetect HasFile's ASCII folding for ordinary ASCII names (the Rust regex engine may also apply Unicode folding). Nosey's file rule is a path predicate and can also match a directory with that basename; it is not the upstream regular-file-only indicator. These are candidate marker rules pending review, not equivalent projectdetect semantics. Suffix predicates use Nosey's file-only extension matcher, with its existing case sensitivity. No new operators.
- Swift uses Package.swift and .podspec; directory-only Xcode indicators are deferred because there is no dedicated subdirectory-only operator. R uses .Rproj; bare DESCRIPTION is ambiguous. Jekyll _config.yml/.yaml and MATLAB .prj are deferred because they are generic markers. Terraform/OpenTofu is a combined marker label, not a claim to distinguish the two tools.
- Do not load project-local detection configurations or a CEL interpreter. Do not auto-register roots, use upstream first-indicator suppression, or add a second walk. Keep all matching evidence and global safety policy.

## Global exclusions

Add 17 relatively specific generated/cache directory names derived from upstream: `.tox`, `.pytest_cache`, `.mypy_cache`, `.ruff_cache`, `.gradle`, `.terraform`, `.swiftpm`, `zig-out`, `zig-cache`, `.zig-cache`, `.astro`, `.gatsby`, `.docusaurus`, `.bundle`, `.jekyll-cache`, `.sass-cache`, `autom4te.cache`. These apply to directory basenames only, at any permitted depth. Prior defaults are unchanged. No registry may add exclusions; user global extra globs still apply. Deliberately do not add ambiguous names such as vendor, bin, obj, public, resources, site, output, out, venv, .build or DerivedData. Defaults remain provisional and can hide legitimate same-named directories; review with benchmarks and user requirements. Emit diagnostic codes for observed built-in or custom pruning without classifying intentional pruning as incomplete. Bump the detection fingerprint so older saved scans become stale.

## Tests

Combined reference-registry regression tests exercise nested multi-technology results, all positive evidence, exact case-insensitive file marker matches, deduplication, directory-only cache pruning, legitimate source directories, explicit custom exclusion, empty-root cancellation and depth limits. Tests are written, not executed; no build or production migration is claimed.
